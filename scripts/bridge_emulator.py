#!/usr/bin/env python3
"""
Apple TV Remote - Android Emulator Bridge for macOS
===================================================
Bridges Apple TV Remote protocol traffic from your physical Wi-Fi network
into the Android TV emulator via ADB port forwarding and macOS mDNS (Bonjour).

Usage:
    python3 scripts/bridge_emulator.py start [-f|--foreground] [-s SERIAL] [--name NAME] [--port PORT]
    python3 scripts/bridge_emulator.py stop
    python3 scripts/bridge_emulator.py restart
    python3 scripts/bridge_emulator.py status
    python3 scripts/bridge_emulator.py logs [-f]
"""

import argparse
import asyncio
import json
import os
import signal
import subprocess
import sys
import time

PID_FILE = "/tmp/atv_bridge_emulator.pid"
STATE_FILE = "/tmp/atv_bridge_emulator.json"
LOG_FILE = "/tmp/atv_bridge_emulator.log"

DEFAULT_DEVICE_NAME = "Android TV Emulator"

# Default Apple TV Protocol Ports
DEFAULT_MRP_PORT = 49152
DEFAULT_COMPANION_PORT = 49153
DEFAULT_AIRPLAY_PORT = 49154

DEVICE_ID = "AA:BB:CC:DD:EE:01"
SERVER_IDENTIFIER = "2E468249-2F22-4416-86C8-50BF22D4F24D"
UNIQUE_ID = SERVER_IDENTIFIER.replace("-", "")
DEVICE_MODEL = "AppleTV5,3"
SOURCE_VERSION = "715.2"

dns_processes = []
target_device = None
device_name = DEFAULT_DEVICE_NAME

def resolve_ports(base_port=None, mrp=None, companion=None, airplay=None):
    if base_port is not None:
        m = mrp if mrp is not None else base_port
        c = companion if companion is not None else (base_port + 1)
        a = airplay if airplay is not None else (base_port + 2)
    else:
        m = mrp if mrp is not None else DEFAULT_MRP_PORT
        c = companion if companion is not None else DEFAULT_COMPANION_PORT
        a = airplay if airplay is not None else DEFAULT_AIRPLAY_PORT
    return m, c, a

def get_target_device(requested_serial=None):
    if requested_serial:
        return requested_serial
    res = subprocess.run(["adb", "devices"], capture_output=True, text=True)
    if res.returncode != 0:
        return None
    devices = []
    for line in res.stdout.strip().splitlines()[1:]:
        parts = line.split()
        if len(parts) >= 2 and parts[1] == "device":
            devices.append(parts[0])
    if not devices:
        return None
    emulators = [d for d in devices if d.startswith("emulator-")]
    if emulators:
        return emulators[0]
    return devices[0]

def adb_cmd(args, serial=None):
    s = serial or target_device
    cmd = ["adb"]
    if s:
        cmd.extend(["-s", s])
    cmd.extend(args)
    return cmd

def get_pids_using_port(port):
    res = subprocess.run(["lsof", "-t", f"-i:{port}"], capture_output=True, text=True)
    if res.returncode == 0 and res.stdout.strip():
        pids = []
        for p in res.stdout.strip().split():
            if p.isdigit():
                pids.append(int(p))
        return list(set(pids))
    return []

def get_process_name(pid):
    res = subprocess.run(["ps", "-p", str(pid), "-o", "comm="], capture_output=True, text=True)
    if res.returncode == 0:
        return res.stdout.strip()
    return "unknown"

def get_running_pid():
    if not os.path.exists(PID_FILE):
        return None
    try:
        with open(PID_FILE, "r") as f:
            pid = int(f.read().strip())
        os.kill(pid, 0)
        return pid
    except (OSError, ValueError):
        # Stale PID file
        if os.path.exists(PID_FILE):
            os.remove(PID_FILE)
        return None

# =========================================================================
# Networking & Proxy Implementation
# =========================================================================

async def pipe(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    try:
        while not reader.at_eof():
            data = await reader.read(4096)
            if not data:
                break
            writer.write(data)
            await writer.drain()
    except Exception:
        pass
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass

def create_proxy(target_port: int, service_name: str):
    async def handler(local_reader: asyncio.StreamReader, local_writer: asyncio.StreamWriter):
        peer = local_writer.get_extra_info("peername")
        print(f"[{service_name}] Connection from {peer} -> forwarding to emulator:{target_port}")
        try:
            remote_reader, remote_writer = await asyncio.open_connection("127.0.0.1", target_port)
            asyncio.create_task(pipe(local_reader, remote_writer))
            asyncio.create_task(pipe(remote_reader, local_writer))
        except Exception as e:
            print(f"[{service_name}] Failed to connect to emulator ({target_port}): {e}")
            local_writer.close()
    return handler

async def handle_airplay(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    peer = writer.get_extra_info("peername")
    print(f"[AirPlay] Ping from {peer}")
    try:
        await reader.read(4096)
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK")
        await writer.drain()
    except Exception:
        pass
    finally:
        try:
            writer.close()
            await writer.wait_closed()
        except Exception:
            pass

def start_mdns_service(name: str, service_type: str, port: int, txt_records: list):
    cmd = ["/usr/bin/dns-sd", "-R", name, service_type, "local.", str(port)] + txt_records
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    dns_processes.append(proc)

def remove_bridge_adb_forwards(target_dev=None):
    res = subprocess.run(["adb", "forward", "--list"], capture_output=True, text=True)
    if res.returncode == 0:
        for line in res.stdout.strip().splitlines():
            parts = line.split()
            if len(parts) >= 3:
                dev = parts[0]
                local_spec = parts[1]
                remote_spec = parts[2]
                if remote_spec in (f"tcp:{DEFAULT_MRP_PORT}", f"tcp:{DEFAULT_COMPANION_PORT}"):
                    if target_dev is None or target_dev == dev:
                        subprocess.run(["adb", "-s", dev, "forward", "--remove", local_spec], capture_output=True)

def cleanup_local_services(serial=None):
    print("\nStopping bridge and cleaning up mDNS & port forwards...")
    for p in dns_processes:
        try:
            p.terminate()
            p.wait(timeout=1)
        except Exception:
            try:
                p.kill()
            except Exception:
                pass
    remove_bridge_adb_forwards(serial)
    print("Cleanup done.")

# =========================================================================
# Bridge Execution Core
# =========================================================================

async def run_bridge(serial=None, name=DEFAULT_DEVICE_NAME, mrp_port=DEFAULT_MRP_PORT, companion_port=DEFAULT_COMPANION_PORT, airplay_port=DEFAULT_AIRPLAY_PORT):
    global target_device, device_name
    device_name = name
    target_device = get_target_device(serial)
    if not target_device:
        print("Error: No ADB devices or emulators found.", file=sys.stderr)
        print("Please start the Android TV emulator or check 'adb devices'.", file=sys.stderr)
        sys.exit(1)

    local_forward_mrp = mrp_port + 10000
    local_forward_companion = companion_port + 10000

    print("=== Apple TV Remote Emulator Bridge ===")
    print(f"Target ADB Device : {target_device}")
    print(f"Advertised Name   : {device_name}")
    print(f"Host Ports        : {mrp_port} (MRP) / {companion_port} (Companion) / {airplay_port} (AirPlay)")

    # Check port conflicts
    for port in (mrp_port, companion_port):
        pids = get_pids_using_port(port)
        my_pid = os.getpid()
        other_pids = [p for p in pids if p != my_pid]
        if other_pids:
            proc_desc = ", ".join(f"{get_process_name(p)} (PID {p})" for p in other_pids)
            print(f"Error: Port {port} is already in use by: {proc_desc}", file=sys.stderr)
            if port == DEFAULT_MRP_PORT:
                print("Tip: Change port via '--port <PORT>' (e.g. '--port 49252') or quit the conflicting application.", file=sys.stderr)
            else:
                print("Please quit the conflicting application first.", file=sys.stderr)
            sys.exit(1)

    # 1. Setup adb port forwards (Mac loopback -> Emulator default ports)
    print(f"1. Configuring ADB forwards (127.0.0.1:{local_forward_mrp} -> emulator:{DEFAULT_MRP_PORT})...")
    res1 = subprocess.run(adb_cmd(["forward", f"tcp:{local_forward_mrp}", f"tcp:{DEFAULT_MRP_PORT}"]), capture_output=True, text=True)
    res2 = subprocess.run(adb_cmd(["forward", f"tcp:{local_forward_companion}", f"tcp:{DEFAULT_COMPANION_PORT}"]), capture_output=True, text=True)
    if res1.returncode != 0 or res2.returncode != 0:
        print(f"Error configuring ADB forward: {res1.stderr} {res2.stderr}", file=sys.stderr)
        sys.exit(1)

    # 2. Start TCP relay servers on 0.0.0.0
    print("2. Starting TCP relays on Mac network interfaces...")
    try:
        server_mrp = await asyncio.start_server(
            create_proxy(local_forward_mrp, "MRP"),
            "0.0.0.0",
            mrp_port
        )
        server_companion = await asyncio.start_server(
            create_proxy(local_forward_companion, "Companion"),
            "0.0.0.0",
            companion_port
        )
        server_airplay = await asyncio.start_server(
            handle_airplay,
            "0.0.0.0",
            airplay_port
        )
    except OSError as e:
        print(f"Error binding port: {e}", file=sys.stderr)
        sys.exit(1)

    # 3. Register mDNS / Bonjour services via macOS mDNSResponder
    print(f"3. Registering Bonjour mDNS services on local Wi-Fi (ports {mrp_port}, {companion_port})...")
    start_mdns_service(
        device_name,
        "_mediaremotetv._tcp",
        mrp_port,
        [
            f"Name={device_name}",
            f"UniqueIdentifier={UNIQUE_ID}",
            "SystemBuildVersion=22K160",
            "LocalAirPlayReceiverPairingIdentity=9C4F2B8A1D3E5F60",
            "ModelName=Apple TV",
            "AllowPairing=YES",
        ]
    )
    start_mdns_service(
        device_name,
        "_companion-link._tcp",
        companion_port,
        [
            "rpMac=1",
            "rpHA=D851F0A4E5C9",
            "rpHN=B7359A9BCBAB",
            f"rpVr={SOURCE_VERSION}",
            f"rpMd={DEVICE_MODEL}",
            "rpFl=0x36782",
            "rpAD=F0E18C86DB60",
            "rpHI=40DB206B32FA",
            f"rpBA={DEVICE_ID}",
        ]
    )
    start_mdns_service(
        device_name,
        "_airplay._tcp",
        airplay_port,
        [
            f"deviceid={DEVICE_ID}",
            "features=0x5A7FFFF7,0x1E",
            "flags=0x44",
            f"model={DEVICE_MODEL}",
            f"srcvers={SOURCE_VERSION}",
            "vv=2",
            f"pi={SERVER_IDENTIFIER}",
            "pk=6b8b4567f85b7f54a3e1c0a93f0a9e2c",
            f"name={device_name}",
        ]
    )

    print("\n" + "=" * 60)
    print("  ✓ Bridge is RUNNING successfully!")
    print(f"  • Device Name : {device_name}")
    print("  • Pairing code: shown on the emulator screen")
    print(f"  • Target      : Android Emulator ({target_device})")
    print(f"  • Local Ports : {mrp_port} (MRP) / {companion_port} (Companion)")
    print("=" * 60)
    print("Instructions:")
    print("1. On iPhone / iPad, open Control Center -> tap Apple TV Remote.")
    print(f"2. You should now see \"{device_name}\" in the device list!")
    print("3. Tap it and enter the code shown on the emulator screen")
    print("4. Press buttons or swipe on iPhone -> watch the Android TV emulator react!")
    print("=" * 60)
    sys.stdout.flush()

    loop = asyncio.get_running_loop()
    stop_event = asyncio.Event()

    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, stop_event.set)

    try:
        await stop_event.wait()
    finally:
        cleanup_local_services(target_device)

# =========================================================================
# CLI Commands: start, stop, restart, status, logs
# =========================================================================

def cmd_start(args):
    pid = get_running_pid()
    if pid:
        print(f"Bridge is already running (PID: {pid}).")
        return

    mrp_port, companion_port, airplay_port = resolve_ports(
        base_port=args.port,
        mrp=args.mrp_port,
        companion=args.companion_port,
        airplay=args.airplay_port,
    )

    # Foreground mode
    if args.foreground:
        with open(PID_FILE, "w") as f:
            f.write(str(os.getpid()))
        try:
            asyncio.run(run_bridge(
                serial=args.serial,
                name=args.name,
                mrp_port=mrp_port,
                companion_port=companion_port,
                airplay_port=airplay_port,
            ))
        finally:
            if os.path.exists(PID_FILE):
                os.remove(PID_FILE)
        return

    # Check device before daemonizing
    dev = get_target_device(args.serial)
    if not dev:
        print("Error: No ADB devices or emulators found.", file=sys.stderr)
        print("Please start the Android TV emulator or check 'adb devices'.", file=sys.stderr)
        sys.exit(1)

    # Check port conflicts
    for port in (mrp_port, companion_port):
        pids = get_pids_using_port(port)
        if pids:
            proc_desc = ", ".join(f"{get_process_name(p)} (PID {p})" for p in pids)
            print(f"Error: Port {port} is already in use by: {proc_desc}", file=sys.stderr)
            if port == DEFAULT_MRP_PORT:
                print("Tip: Change port via '--port <PORT>' (e.g. '--port 49252') or quit the conflicting application.", file=sys.stderr)
            else:
                print("Please quit the conflicting application first.", file=sys.stderr)
            sys.exit(1)

    # Launch daemon in background
    cmd = [
        sys.executable,
        os.path.abspath(__file__),
        "run",
        "--name", args.name,
        "--mrp-port", str(mrp_port),
        "--companion-port", str(companion_port),
        "--airplay-port", str(airplay_port),
    ]
    if args.serial:
        cmd.extend(["--serial", args.serial])

    with open(LOG_FILE, "a") as log_f:
        log_f.write(f"\n--- Bridge started at {time.strftime('%Y-%m-%d %H:%M:%S')} ---\n")
        log_f.flush()
        proc = subprocess.Popen(
            cmd,
            stdout=log_f,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )

    # Write PID & state
    with open(PID_FILE, "w") as f:
        f.write(str(proc.pid))

    state = {
        "pid": proc.pid,
        "serial": dev,
        "name": args.name,
        "start_time": time.time(),
        "mrp_port": mrp_port,
        "companion_port": companion_port,
        "airplay_port": airplay_port,
    }
    with open(STATE_FILE, "w") as f:
        json.dump(state, f, indent=2)

    # Verify if process stays alive
    time.sleep(1.0)
    if proc.poll() is not None:
        print("Failed to start bridge. Check log output below:", file=sys.stderr)
        if os.path.exists(LOG_FILE):
            with open(LOG_FILE, "r") as f:
                lines = f.readlines()
                print("".join(lines[-15:]), file=sys.stderr)
        if os.path.exists(PID_FILE):
            os.remove(PID_FILE)
        sys.exit(1)

    print(f"✓ Bridge started in background (PID: {proc.pid})")
    print(f"  • Target Device : {dev}")
    print(f"  • Device Name   : {args.name}")
    print(f"  • Host Ports    : {mrp_port} (MRP) / {companion_port} (Companion)")
    print(f"  • Log File      : {LOG_FILE}")
    print("\nRun 'python3 scripts/bridge_emulator.py status' or 'stop' anytime.")

def cmd_stop(args):
    pid = get_running_pid()
    state = {}
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE, "r") as f:
                state = json.load(f)
        except Exception:
            pass

    serial = state.get("serial")

    if pid:
        print(f"Stopping bridge process (PID: {pid})...")
        try:
            os.kill(pid, signal.SIGTERM)
            for _ in range(30):
                time.sleep(0.1)
                try:
                    os.kill(pid, 0)
                except OSError:
                    break
            else:
                os.kill(pid, signal.SIGKILL)
        except OSError:
            pass

    # Clean up pid & state
    if os.path.exists(PID_FILE):
        os.remove(PID_FILE)
    if os.path.exists(STATE_FILE):
        os.remove(STATE_FILE)

    # Clean up any orphaned dns-sd processes
    subprocess.run(["pkill", "-f", "dns-sd -R .*_mediaremotetv"], capture_output=True)
    subprocess.run(["pkill", "-f", "dns-sd -R .*_companion-link"], capture_output=True)
    subprocess.run(["pkill", "-f", "dns-sd -R .*_airplay"], capture_output=True)

    # Clean up ADB forwards
    remove_bridge_adb_forwards(serial)

    print("✓ Bridge stopped and all port forwards removed.")

def cmd_status(args):
    pid = get_running_pid()
    state = {}
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE, "r") as f:
                state = json.load(f)
        except Exception:
            pass

    mrp_p = state.get("mrp_port", DEFAULT_MRP_PORT)
    comp_p = state.get("companion_port", DEFAULT_COMPANION_PORT)
    air_p = state.get("airplay_port", DEFAULT_AIRPLAY_PORT)

    print("=== Apple TV Remote Bridge Status ===")
    if pid:
        start_t = state.get("start_time")
        uptime = f"{int(time.time() - start_t)}s" if start_t else "unknown"
        print(f"  • Status        : RUNNING (PID: {pid}, uptime: {uptime})")
        print(f"  • Target Device : {state.get('serial', 'auto')}")
        print(f"  • Device Name   : {state.get('name', DEFAULT_DEVICE_NAME)}")
        print(f"  • Log File      : {LOG_FILE}")
    else:
        print("  • Status        : STOPPED")

    # ADB forwards status
    res = subprocess.run(["adb", "forward", "--list"], capture_output=True, text=True)
    fwds = [line for line in res.stdout.strip().splitlines() if f"tcp:{DEFAULT_MRP_PORT}" in line or f"tcp:{DEFAULT_COMPANION_PORT}" in line]
    print("  • ADB Forwards  :")
    if fwds:
        for fwd in fwds:
            print(f"      {fwd}")
    else:
        print("      (None)")

    # Port status
    print("  • Port Listeners:")
    for port, label in [(mrp_p, "MRP"), (comp_p, "Companion"), (air_p, "AirPlay")]:
        pids = get_pids_using_port(port)
        if pids:
            desc = ", ".join(f"{get_process_name(p)} (PID {p})" for p in pids)
            print(f"      Port {port} ({label}): {desc}")
        else:
            print(f"      Port {port} ({label}): Not listening")

def cmd_restart(args):
    print("Restarting bridge...")
    cmd_stop(args)
    time.sleep(0.5)
    cmd_start(args)

def cmd_logs(args):
    if not os.path.exists(LOG_FILE):
        print(f"No log file found at {LOG_FILE}.")
        return
    cmd = ["tail"]
    if args.follow:
        cmd.append("-f")
    cmd.extend(["-n", str(args.lines), LOG_FILE])
    try:
        subprocess.run(cmd)
    except KeyboardInterrupt:
        pass

# =========================================================================
# Main Entry Point
# =========================================================================

def add_port_arguments(parser):
    parser.add_argument("--port", type=int, help="Base host port (MRP: port, Companion: port+1, AirPlay: port+2)")
    parser.add_argument("--mrp-port", type=int, help=f"Host MRP port (default: {DEFAULT_MRP_PORT})")
    parser.add_argument("--companion-port", type=int, help=f"Host Companion Link port (default: {DEFAULT_COMPANION_PORT})")
    parser.add_argument("--airplay-port", type=int, help=f"Host AirPlay dummy port (default: {DEFAULT_AIRPLAY_PORT})")

def main():
    parser = argparse.ArgumentParser(
        description="Apple TV Remote - Android Emulator Bridge CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command", help="Available subcommands")

    # start
    p_start = subparsers.add_parser("start", help="Start the bridge in background or foreground")
    p_start.add_argument("-f", "--foreground", action="store_true", help="Run in foreground (interactive logs)")
    p_start.add_argument("-s", "--serial", help="ADB device serial (defaults to auto-detect emulator)")
    p_start.add_argument("--name", default=DEFAULT_DEVICE_NAME, help="Device name to advertise via mDNS")
    add_port_arguments(p_start)

    # stop
    subparsers.add_parser("stop", help="Stop the running bridge and clean up mDNS & port forwards")

    # restart
    p_restart = subparsers.add_parser("restart", help="Restart the bridge")
    p_restart.add_argument("-s", "--serial", help="ADB device serial")
    p_restart.add_argument("--name", default=DEFAULT_DEVICE_NAME, help="Device name to advertise")
    p_restart.add_argument("-f", "--foreground", action="store_true", help="Run restarted bridge in foreground")
    add_port_arguments(p_restart)

    # status
    subparsers.add_parser("status", help="Show running status, ports, and ADB forwards")

    # logs
    p_logs = subparsers.add_parser("logs", help="View bridge log output")
    p_logs.add_argument("-f", "--follow", action="store_true", help="Follow log output (tail -f)")
    p_logs.add_argument("-n", "--lines", type=int, default=30, help="Number of lines to show (default: 30)")

    # internal worker subcommand: run
    p_run = subparsers.add_parser("run", help=argparse.SUPPRESS)
    p_run.add_argument("-s", "--serial", help="ADB serial")
    p_run.add_argument("--name", default=DEFAULT_DEVICE_NAME, help="mDNS device name")
    add_port_arguments(p_run)

    args = parser.parse_args()

    if not args.command:
        cmd_status(args)
        print("\nTip: Use 'python3 scripts/bridge_emulator.py start' to start the bridge.")
        return

    if args.command == "start":
        cmd_start(args)
    elif args.command == "stop":
        cmd_stop(args)
    elif args.command == "restart":
        cmd_restart(args)
    elif args.command == "status":
        cmd_status(args)
    elif args.command == "logs":
        cmd_logs(args)
    elif args.command == "run":
        mrp_p, comp_p, air_p = resolve_ports(
            base_port=args.port,
            mrp=args.mrp_port,
            companion=args.companion_port,
            airplay=args.airplay_port,
        )
        asyncio.run(run_bridge(
            serial=args.serial,
            name=args.name,
            mrp_port=mrp_p,
            companion_port=comp_p,
            airplay_port=air_p,
        ))

if __name__ == "__main__":
    main()

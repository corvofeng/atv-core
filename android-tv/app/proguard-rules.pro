# Keep native methods and classes used by JNI
-keepclassmembers class * {
    native <methods>;
}

-keep class com.corvofeng.fakeatv.AtvNative** { *; }
-keep interface com.corvofeng.fakeatv.AtvNative$** { *; }
-keep class com.corvofeng.fakeatv.DeviceInfo** { *; }
-keep class com.corvofeng.fakeatv.AtvService** { *; }
-keep class com.corvofeng.fakeatv.AtvAccessibilityService** { *; }
-keep class com.corvofeng.fakeatv.BootReceiver** { *; }
-keep class com.corvofeng.fakeatv.MainActivity** { *; }
# Started by name through app_process, outside the app
-keep class com.corvofeng.fakeatv.KeyInjector { public static void main(java.lang.String[]); }

# Keep annotations & signature
-keepattributes *Annotation*,Signature,InnerClasses,EnclosingMethod

# Leanback / AndroidX
-dontwarn androidx.leanback.**

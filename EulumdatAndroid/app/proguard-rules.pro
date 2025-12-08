# Add project specific ProGuard rules here.

# Keep uniffi generated code
-keep class uniffi.** { *; }
-keep class eu.trahe.eulumdat.uniffi.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

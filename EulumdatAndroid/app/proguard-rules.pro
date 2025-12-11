# Add project specific ProGuard rules here.

# Keep uniffi generated code
-keep class uniffi.** { *; }
-keep class eu.trahe.eulumdat.uniffi.** { *; }

# Keep JNA classes
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-dontwarn com.sun.jna.**

# JNA references AWT classes that don't exist on Android - ignore them
-dontwarn java.awt.**
-dontwarn sun.awt.**

# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

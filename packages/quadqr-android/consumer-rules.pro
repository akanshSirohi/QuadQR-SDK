# QuadQR's JNI symbols are bound to this exact class name. Keep it stable in
# consuming applications even when R8/ProGuard is enabled.
-keep class io.github.akanshsirohi.quadqr.QuadQRNative { *; }

# Do not rename or strip native method declarations.
-keepclasseswithmembernames class * {
    native <methods>;
}

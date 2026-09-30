package io.github.akanshsirohi.quadqr;
public enum StressDistortion {
    BLUR("blur"), MOTION_BLUR("motionBlur"), LOW_EXPOSURE("lowExposure"), HIGH_EXPOSURE("highExposure"),
    SHADOW("shadow"), GLARE("glare"), WARM_CAST("warmCast"), COOL_CAST("coolCast"),
    CONTRAST_LOSS("contrastLoss"), SENSOR_NOISE("sensorNoise"), JPEG_LIKE("jpegLike"), DOWNSCALE("downscale"),
    PERSPECTIVE("perspective"), PERSPECTIVE_3D("perspective3d"), ROTATION("rotation");
    final String value; StressDistortion(String value){this.value=value;}
}

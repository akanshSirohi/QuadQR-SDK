import 'dart:typed_data';

enum EccLevel { l, m, q, h }
enum CellEncoding { rgbw, highDensity }
enum CompressionMode { none, auto, smart, lz, deflate, brotli }
enum CompressionAlgorithm { lz, deflate, brotli }
enum RenderStyle { classic, depth, soft, inset }
enum RenderMode { screen, print }
enum ReliabilitySuite { quick, full, extreme }
enum PerspectiveAxis { yaw, pitch, roll }
enum StressDistortion { blur, motionBlur, lowExposure, highExposure, shadow, glare, warmCast, coolCast, contrastLoss, sensorNoise, jpegLike, downscale, perspective, perspective3d, rotation }

String _enumName(Object value) {
  if (value is EccLevel) return value.name.toUpperCase();
  if (value is CellEncoding) return value == CellEncoding.highDensity ? 'triangle16' : 'rgbw';
  return (value as dynamic).name as String;
}

class SecurityOptions {
  final String mode; final String? password; final Uint8List? key; final int? iterations; final Uint8List? keyId; final bool disableAutoKeyId;
  const SecurityOptions._(this.mode,{this.password,this.key,this.iterations,this.keyId,this.disableAutoKeyId=false});
  factory SecurityOptions.password(String password,{int iterations=210000,Uint8List? keyId})=>SecurityOptions._('password',password:password,iterations:iterations,keyId:keyId);
  factory SecurityOptions.rawKey(Uint8List key,{Uint8List? keyId,bool disableAutoKeyId=false}){if(key.length!=32)throw ArgumentError('raw key must contain 32 bytes');return SecurityOptions._('raw-key',key:key,keyId:keyId,disableAutoKeyId:disableAutoKeyId);}
  Map<String,dynamic> toMap()=>{ 'mode':mode, if(password!=null)'password':password, if(iterations!=null)'iterations':iterations, if(key!=null)'key':key!.toList(), if(keyId!=null)'keyId':keyId!.toList(), if(mode=='raw-key')'disableAutoKeyId':disableAutoKeyId };
}

class SigningOptions {
  final Uint8List privateKeySeed; final Uint8List? keyId; final bool embedPublicKey;
  const SigningOptions(this.privateKeySeed,{this.keyId,this.embedPublicKey=false});
  Map<String,dynamic> toMap()=>{'privateKeySeed':privateKeySeed.toList(),if(keyId!=null)'keyId':keyId!.toList(),'embedPublicKey':embedPublicKey};
}

class EncodeOptions {
  final EccLevel ecc; final CellEncoding cellEncoding; final CompressionMode compression; final int? compressionLevel,version,minVersion,maxVersion; final SecurityOptions? security; final SigningOptions? signing;
  const EncodeOptions({this.ecc=EccLevel.m,this.cellEncoding=CellEncoding.rgbw,this.compression=CompressionMode.none,this.compressionLevel,this.version,this.minVersion,this.maxVersion,this.security,this.signing});
  Map<String,dynamic> toMap()=>{'ecc':_enumName(ecc),'cellEncoding':_enumName(cellEncoding),'highDensity':cellEncoding==CellEncoding.highDensity,'compression':compression.name,if(compressionLevel!=null)'compressionLevel':compressionLevel,if(version!=null)'version':version,if(minVersion!=null)'minVersion':minVersion,if(maxVersion!=null)'maxVersion':maxVersion,if(security!=null)'security':security!.toMap(),if(signing!=null)'signing':signing!.toMap()};
}

class DecodeOptions {
  final double? structuralTolerance,maxErasureConfidence,softConfidenceThreshold; final bool? softDecoding; final int? softMaxCells,softPairCells; final List<List<double>>? confidence; final List<List<int>>? alternatives;
  const DecodeOptions({this.structuralTolerance,this.maxErasureConfidence,this.softDecoding,this.softConfidenceThreshold,this.softMaxCells,this.softPairCells,this.confidence,this.alternatives});
  Map<String,dynamic> toMap()=>{if(structuralTolerance!=null)'structuralTolerance':structuralTolerance,if(maxErasureConfidence!=null)'maxErasureConfidence':maxErasureConfidence,if(softDecoding!=null)'softDecoding':softDecoding,if(softConfidenceThreshold!=null)'softConfidenceThreshold':softConfidenceThreshold,if(softMaxCells!=null)'softMaxCells':softMaxCells,if(softPairCells!=null)'softPairCells':softPairCells,if(confidence!=null)'confidence':confidence,if(alternatives!=null)'alternatives':alternatives};
}

class ScanOptions {
  final int? minVersion,
      maxVersion,
      maxFinderCandidates,
      locatorMaxDimension,
      workingMaxDimension,
      softMaxCells,
      softPairCells;
  final double? structuralTolerance,
      sampleRadius,
      highDensitySampleRadius,
      maxErasureConfidence,
      softConfidenceThreshold;
  final bool? axisAlignedFallback,
      finderRecovery,
      rotationRecovery,
      softDecoding;

  const ScanOptions({
    this.minVersion,
    this.maxVersion,
    this.maxFinderCandidates,
    this.locatorMaxDimension,
    this.workingMaxDimension,
    this.softMaxCells,
    this.softPairCells,
    this.structuralTolerance,
    this.sampleRadius,
    this.highDensitySampleRadius,
    this.maxErasureConfidence,
    this.softConfidenceThreshold,
    this.axisAlignedFallback,
    this.finderRecovery,
    this.rotationRecovery,
    this.softDecoding,
  });

  Map<String, dynamic> toMap() => {
        if (minVersion != null) 'minVersion': minVersion,
        if (maxVersion != null) 'maxVersion': maxVersion,
        if (maxFinderCandidates != null)
          'maxFinderCandidates': maxFinderCandidates,
        if (locatorMaxDimension != null)
          'locatorMaxDimension': locatorMaxDimension,
        if (workingMaxDimension != null)
          'workingMaxDimension': workingMaxDimension,
        if (structuralTolerance != null)
          'structuralTolerance': structuralTolerance,
        if (axisAlignedFallback != null)
          'axisAlignedFallback': axisAlignedFallback,
        if (finderRecovery != null) 'finderRecovery': finderRecovery,
        if (rotationRecovery != null) 'rotationRecovery': rotationRecovery,
        if (sampleRadius != null) 'sampleRadius': sampleRadius,
        if (highDensitySampleRadius != null)
          'highDensitySampleRadius': highDensitySampleRadius,
        if (maxErasureConfidence != null)
          'maxErasureConfidence': maxErasureConfidence,
        if (softDecoding != null) 'softDecoding': softDecoding,
        if (softConfidenceThreshold != null)
          'softConfidenceThreshold': softConfidenceThreshold,
        if (softMaxCells != null) 'softMaxCells': softMaxCells,
        if (softPairCells != null) 'softPairCells': softPairCells,
      };
}

class Palette {
  final String? black,white,red,green,blue; const Palette({this.black,this.white,this.red,this.green,this.blue});
  Map<String,dynamic> toMap()=>{if(black!=null)'black':black,if(white!=null)'white':white,if(red!=null)'red':red,if(green!=null)'green':green,if(blue!=null)'blue':blue};
}
class LogoOptions {
  final double? size,padding,radius; final bool autoSize,clearBackground; final String? backgroundColor;
  const LogoOptions({this.size,this.autoSize=false,this.padding,this.radius,this.clearBackground=false,this.backgroundColor});
  Map<String,dynamic> toMap()=>{if(autoSize)'size':'auto' else if(size!=null)'size':size,if(padding!=null)'padding':padding,if(radius!=null)'radius':radius,'clearBackground':clearBackground,if(backgroundColor!=null)'backgroundColor':backgroundColor};
}
class RenderOptions {
  final int? moduleSize,imageSize,quietZone; final RenderMode mode; final RenderStyle style; final bool allowStyledPrint; final Palette? palette; final LogoOptions? logo;
  const RenderOptions({this.moduleSize,this.imageSize,this.quietZone,this.mode=RenderMode.screen,this.style=RenderStyle.classic,this.allowStyledPrint=false,this.palette,this.logo});
  Map<String,dynamic> toMap()=>{'mode':mode.name,'style':style.name,if(moduleSize!=null)'moduleSize':moduleSize,if(imageSize!=null)'imageSize':imageSize,if(quietZone!=null)'quietZone':quietZone,'allowStyledPrint':allowStyledPrint,if(palette!=null)'palette':palette!.toMap(),if(logo!=null)'logo':logo!.toMap()};
}
class PrintOptions { final int quietZone; final double physicalSizeMm,dpi,minimumModuleMm; const PrintOptions({this.quietZone=4,this.physicalSizeMm=35,this.dpi=300,this.minimumModuleMm=.40}); Map<String,dynamic> toMap()=>{'quietZone':quietZone,'physicalSizeMm':physicalSizeMm,'dpi':dpi,'minimumModuleMm':minimumModuleMm}; }
class StressOptions { final StressDistortion type; final double severity,pitchDegrees,yawDegrees,rollDegrees; const StressOptions(this.type,{this.severity=.5,this.pitchDegrees=0,this.yawDegrees=0,this.rollDegrees=0}); Map<String,dynamic> toMap()=>{'type':type.name,'severity':severity,'perspective':{'pitchDegrees':pitchDegrees,'yawDegrees':yawDegrees,'rollDegrees':rollDegrees}}; }

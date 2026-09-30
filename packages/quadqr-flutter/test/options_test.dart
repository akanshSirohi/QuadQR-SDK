import 'package:flutter_test/flutter_test.dart';
import 'package:quadqr_flutter/quadqr.dart';

void main() {
  test('encode options map developer-facing enums correctly', () {
    const options = EncodeOptions(
      ecc: EccLevel.q,
      cellEncoding: CellEncoding.highDensity,
      compression: CompressionMode.auto,
    );

    final map = options.toMap();
    expect(map['ecc'], 'Q');
    expect(map['cellEncoding'], 'triangle16');
    expect(map['highDensity'], true);
    expect(map['compression'], 'auto');
  });


  test('scan options expose the full working-frame cap', () {
    const options = ScanOptions(workingMaxDimension: 960);
    expect(options.toMap()['workingMaxDimension'], 960);
  });
}

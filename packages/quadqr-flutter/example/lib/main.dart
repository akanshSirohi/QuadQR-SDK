import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:quadqr_flutter/quadqr.dart';

void main() {
  runApp(const QuadQRExampleApp());
}

class QuadQRExampleApp extends StatelessWidget {
  const QuadQRExampleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'QuadQR Flutter',
      theme: ThemeData(useMaterial3: true),
      home: const QuadQRExamplePage(),
    );
  }
}

class QuadQRExamplePage extends StatefulWidget {
  const QuadQRExamplePage({super.key});

  @override
  State<QuadQRExamplePage> createState() => _QuadQRExamplePageState();
}

class _QuadQRExamplePageState extends State<QuadQRExamplePage> {
  final _controller = TextEditingController(text: 'Hello from QuadQR Flutter');
  Uint8List? _png;
  String? _error;

  void _generate() {
    try {
      final code = QuadQR.instance.encodeText(
        _controller.text,
        options: const EncodeOptions(
          ecc: EccLevel.m,
          compression: CompressionMode.auto,
        ),
      );
      final png = QuadQR.instance.renderPng(
        code,
        options: const RenderOptions(imageSize: 720),
      );
      setState(() {
        _png = png;
        _error = null;
      });
    } catch (error) {
      setState(() {
        _error = error.toString();
      });
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('QuadQR Flutter')),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          TextField(
            controller: _controller,
            decoration: const InputDecoration(
              labelText: 'Text to encode',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 12),
          FilledButton(
            onPressed: _generate,
            child: const Text('Generate QuadQR'),
          ),
          if (_error != null) ...[
            const SizedBox(height: 16),
            Text(_error!, style: const TextStyle(color: Colors.red)),
          ],
          if (_png != null) ...[
            const SizedBox(height: 24),
            Center(child: Image.memory(_png!, width: 320, height: 320)),
          ],
        ],
      ),
    );
  }
}

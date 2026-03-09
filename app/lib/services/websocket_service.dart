import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:web_socket_channel/web_socket_channel.dart';
import '../models/models.dart';

/// WebSocket service for real-time communication with the Secretarybird server.
/// Mirrors the old Electron WebSocket client: auto-reconnect with exponential backoff,
/// stream_start/stream_chunk/stream_end protocol.
class WebSocketService extends ChangeNotifier {
  WebSocketChannel? _channel;
  Timer? _reconnectTimer;
  int _reconnectAttempts = 0;
  static const int _maxReconnectDelay = 30;

  String _serverUrl;
  bool _connected = false;
  bool _openclawAvailable = false;
  String _streamBuffer = '';
  bool _isStreaming = false;

  // Callbacks
  final List<void Function(WSMessage)> _listeners = [];
  final List<void Function(String)> _streamListeners = [];
  final List<void Function()> _streamEndListeners = [];

  bool get connected => _connected;
  bool get openclawAvailable => _openclawAvailable;
  bool get isStreaming => _isStreaming;
  String get streamBuffer => _streamBuffer;

  WebSocketService({String serverUrl = 'ws://127.0.0.1:8000/ws/chat'})
      : _serverUrl = serverUrl;

  /// Connect to the server.
  void connect({String? url}) {
    if (url != null) _serverUrl = url;
    _doConnect();
  }

  /// Disconnect from the server.
  void disconnect() {
    _reconnectTimer?.cancel();
    _channel?.sink.close();
    _channel = null;
    _connected = false;
    notifyListeners();
  }

  /// Send a chat message to the server.
  void sendMessage(String content, {List<Map<String, String>>? history}) {
    if (!_connected) return;

    final payload = {
      'content': content,
      'history': history ?? [],
    };
    _channel?.sink.add(jsonEncode(payload));
    _streamBuffer = '';
    _isStreaming = false;
    notifyListeners();
  }

  /// Register a listener for all WebSocket messages.
  void addMessageListener(void Function(WSMessage) listener) {
    _listeners.add(listener);
  }

  /// Register a listener for streaming chunks.
  void addStreamListener(void Function(String) listener) {
    _streamListeners.add(listener);
  }

  /// Register a listener for stream completion.
  void addStreamEndListener(void Function() listener) {
    _streamEndListeners.add(listener);
  }

  void _doConnect() {
    try {
      _channel = WebSocketChannel.connect(Uri.parse(_serverUrl));
      _channel!.stream.listen(
        _onMessage,
        onError: _onError,
        onDone: _onDone,
      );
      _connected = true;
      _reconnectAttempts = 0;
      notifyListeners();
      debugPrint('[ws] connected to $_serverUrl');
    } catch (e) {
      debugPrint('[ws] connection failed: $e');
      _scheduleReconnect();
    }
  }

  void _onMessage(dynamic raw) {
    try {
      final json = jsonDecode(raw as String) as Map<String, dynamic>;
      final msg = WSMessage.fromJson(json);

      switch (msg.type) {
        case 'status':
          if (msg.data is Map) {
            _openclawAvailable = (msg.data as Map)['available'] == true;
          }
          notifyListeners();

        case 'stream_start':
          _isStreaming = true;
          _streamBuffer = '';
          notifyListeners();

        case 'stream_chunk':
          if (msg.content != null) {
            _streamBuffer += msg.content!;
            for (final listener in _streamListeners) {
              listener(msg.content!);
            }
            notifyListeners();
          }

        case 'stream_end':
          _isStreaming = false;
          for (final listener in _streamEndListeners) {
            listener();
          }
          notifyListeners();

        case 'graph_diff':
          // Graph diff broadcast - notify all listeners
          break;

        case 'error':
          debugPrint('[ws] server error: ${msg.content}');
      }

      for (final listener in _listeners) {
        listener(msg);
      }
    } catch (e) {
      debugPrint('[ws] parse error: $e');
    }
  }

  void _onError(dynamic error) {
    debugPrint('[ws] error: $error');
    _connected = false;
    notifyListeners();
    _scheduleReconnect();
  }

  void _onDone() {
    debugPrint('[ws] disconnected');
    _connected = false;
    notifyListeners();
    _scheduleReconnect();
  }

  void _scheduleReconnect() {
    _reconnectTimer?.cancel();
    final delay = (_reconnectAttempts * 2).clamp(1, _maxReconnectDelay);
    _reconnectAttempts++;
    debugPrint('[ws] reconnecting in ${delay}s (attempt $_reconnectAttempts)');
    _reconnectTimer = Timer(Duration(seconds: delay), _doConnect);
  }

  @override
  void dispose() {
    disconnect();
    _listeners.clear();
    _streamListeners.clear();
    _streamEndListeners.clear();
    super.dispose();
  }
}

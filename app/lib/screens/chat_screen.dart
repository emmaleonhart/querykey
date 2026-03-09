import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../services/websocket_service.dart';

/// Chat screen - the primary interaction surface.
/// Mirrors the old Electron chat interface: send messages, stream responses from OpenClaw.
class ChatScreen extends StatefulWidget {
  const ChatScreen({super.key});

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  final List<_ChatMessage> _messages = [];

  @override
  void initState() {
    super.initState();
    final ws = context.read<WebSocketService>();
    ws.addStreamEndListener(_onStreamEnd);
  }

  void _onStreamEnd() {
    final ws = context.read<WebSocketService>();
    if (ws.streamBuffer.isNotEmpty) {
      setState(() {
        // Replace the streaming placeholder with the final message
        if (_messages.isNotEmpty && _messages.last.isStreaming) {
          _messages.last = _ChatMessage(
            content: ws.streamBuffer,
            isUser: false,
            isStreaming: false,
          );
        }
      });
    }
  }

  void _sendMessage() {
    final text = _controller.text.trim();
    if (text.isEmpty) return;

    final ws = context.read<WebSocketService>();

    setState(() {
      _messages.add(_ChatMessage(content: text, isUser: true));
      _messages
          .add(_ChatMessage(content: '', isUser: false, isStreaming: true));
    });

    // Build history from previous messages
    final history = <Map<String, String>>[];
    for (final msg in _messages) {
      if (!msg.isStreaming) {
        history.add({
          'role': msg.isUser ? 'user' : 'assistant',
          'content': msg.content,
        });
      }
    }

    ws.sendMessage(text, history: history);
    _controller.clear();
    _scrollToBottom();
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Consumer<WebSocketService>(
      builder: (context, ws, _) {
        // Update streaming message in real-time
        if (_messages.isNotEmpty &&
            _messages.last.isStreaming &&
            ws.streamBuffer.isNotEmpty) {
          _messages.last = _ChatMessage(
            content: ws.streamBuffer,
            isUser: false,
            isStreaming: true,
          );
        }

        return Column(
          children: [
            // Connection status bar
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              color: ws.connected
                  ? const Color(0xFF1B5E20)
                  : const Color(0xFF4E342E),
              child: Row(
                children: [
                  Icon(
                    ws.connected ? Icons.cloud_done : Icons.cloud_off,
                    size: 16,
                    color: Colors.white70,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    ws.connected ? 'Connected' : 'Disconnected',
                    style: const TextStyle(
                        color: Colors.white70, fontSize: 12),
                  ),
                  const Spacer(),
                  if (ws.connected)
                    Text(
                      ws.openclawAvailable
                          ? 'OpenClaw: ready'
                          : 'OpenClaw: offline',
                      style: TextStyle(
                        color: ws.openclawAvailable
                            ? Colors.green[200]
                            : Colors.orange[200],
                        fontSize: 12,
                      ),
                    ),
                ],
              ),
            ),

            // Messages
            Expanded(
              child: ListView.builder(
                controller: _scrollController,
                padding: const EdgeInsets.all(16),
                itemCount: _messages.length,
                itemBuilder: (context, index) {
                  final msg = _messages[index];
                  return _MessageBubble(message: msg);
                },
              ),
            ),

            // Input
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surface,
                border: Border(
                  top: BorderSide(
                    color: Theme.of(context).dividerColor,
                  ),
                ),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _controller,
                      decoration: const InputDecoration(
                        hintText: 'Ask the secretary...',
                        border: OutlineInputBorder(),
                        contentPadding: EdgeInsets.symmetric(
                            horizontal: 16, vertical: 12),
                      ),
                      onSubmitted: (_) => _sendMessage(),
                      maxLines: null,
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton.filled(
                    onPressed:
                        ws.isStreaming ? null : _sendMessage,
                    icon: const Icon(Icons.send),
                  ),
                ],
              ),
            ),
          ],
        );
      },
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    _scrollController.dispose();
    super.dispose();
  }
}

class _ChatMessage {
  String content;
  final bool isUser;
  bool isStreaming;

  _ChatMessage({
    required this.content,
    required this.isUser,
    this.isStreaming = false,
  });
}

class _MessageBubble extends StatelessWidget {
  final _ChatMessage message;

  const _MessageBubble({required this.message});

  @override
  Widget build(BuildContext context) {
    final isUser = message.isUser;
    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 4),
        padding: const EdgeInsets.all(12),
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.75,
        ),
        decoration: BoxDecoration(
          color: isUser
              ? Theme.of(context).colorScheme.primary
              : Theme.of(context).colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.circular(12),
        ),
        child: message.isStreaming && message.content.isEmpty
            ? const SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : SelectableText(
                message.content,
                style: TextStyle(
                  color: isUser
                      ? Theme.of(context).colorScheme.onPrimary
                      : Theme.of(context).colorScheme.onSurface,
                ),
              ),
      ),
    );
  }
}

// Rainy Cowork - Streaming Chat Component (PHASE 3)
// UI component for demonstrating streaming chat capabilities

import React, { useState, useRef, useEffect } from 'react';
import { useStreaming } from '../../hooks/useStreaming';
import { useAIProvider } from '../../hooks/useAIProvider';
import type { ChatCompletionRequestDto } from '../../services/tauri';

export function StreamingChat() {
    const { providers, defaultProvider } = useAIProvider();
    const {
        isStreaming,
        error,
        chunks,
        fullText,
        streamChat,
        stopStreaming,
        resetStream,
    } = useStreaming();

    const [messages, setMessages] = useState<Array<{ role: string; content: string }>>([]);
    const [input, setInput] = useState('');
    const [temperature, setTemperature] = useState(0.7);
    const [maxTokens, setMaxTokens] = useState(1024);
    const [selectedProvider, setSelectedProvider] = useState<string | undefined>(undefined);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (defaultProvider) {
            setSelectedProvider(defaultProvider.id);
        }
    }, [defaultProvider]);

    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages, chunks]);

    const handleSend = async () => {
        if (!input.trim() || isStreaming) return;

        const userMessage = { role: 'user', content: input };
        setMessages(prev => [...prev, userMessage]);
        setInput('');

        const request: ChatCompletionRequestDto = {
            provider_id: selectedProvider,
            messages: [...messages, userMessage].map(m => ({
                role: m.role,
                content: m.content,
            })),
            model: undefined,
            temperature,
            max_tokens: maxTokens,
            top_p: undefined,
            frequency_penalty: undefined,
            presence_penalty: undefined,
            stop: undefined,
            stream: true,
        };

        try {
            await streamChat(request, (chunk) => {
                // Chunk is handled by useStreaming hook
            });
        } catch (err) {
            console.error('Streaming failed:', err);
            setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${err}` }]);
        }
    };

    const handleStop = () => {
        stopStreaming();
    };

    const handleReset = () => {
        resetStream();
        setMessages([]);
    };

    const formatTimestamp = () => {
        const now = new Date();
        return now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    };

    return (
        <div className="backdrop-blur-md bg-white/80 rounded-xl shadow-lg p-6">
            <div className="flex items-center justify-between mb-6">
                <h2 className="text-2xl font-bold">Streaming Chat</h2>
                <div className="flex items-center gap-4">
                    {isStreaming && (
                        <button
                            onClick={handleStop}
                            className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600"
                        >
                            Stop Streaming
                        </button>
                    )}
                    <button
                        onClick={handleReset}
                        className="px-4 py-2 bg-gray-500 text-white rounded-lg hover:bg-gray-600"
                    >
                        Reset
                    </button>
                </div>
            </div>

            {error && (
                <div className="mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg">
                    {error}
                </div>
            )}

            {/* Provider Selection */}
            <div className="mb-4">
                <label className="block text-sm font-medium text-gray-700 mb-1">
                    Provider
                </label>
                <select
                    value={selectedProvider}
                    onChange={(e) => setSelectedProvider(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                    disabled={isStreaming}
                >
                    <option value="">Default Provider</option>
                    {providers.map(provider => (
                        <option key={provider.id} value={provider.id}>
                            {provider.id} ({provider.provider_type})
                        </option>
                    ))}
                </select>
            </div>

            {/* Settings */}
            <div className="mb-4 grid grid-cols-2 gap-4">
                <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                        Temperature: {temperature.toFixed(1)}
                    </label>
                    <input
                        type="range"
                        min="0"
                        max="2"
                        step="0.1"
                        value={temperature}
                        onChange={(e) => setTemperature(parseFloat(e.target.value))}
                        className="w-full"
                        disabled={isStreaming}
                    />
                </div>
                <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                        Max Tokens: {maxTokens}
                    </label>
                    <input
                        type="range"
                        min="100"
                        max="4096"
                        step="100"
                        value={maxTokens}
                        onChange={(e) => setMaxTokens(parseInt(e.target.value))}
                        className="w-full"
                        disabled={isStreaming}
                    />
                </div>
            </div>

            {/* Chat Messages */}
            <div className="mb-4 h-96 overflow-y-auto border border-gray-200 rounded-lg p-4 bg-gray-50">
                {messages.length === 0 ? (
                    <div className="text-center py-12 text-gray-500">
                        <p className="text-lg mb-2">No messages yet</p>
                        <p className="text-sm">Start a conversation by typing a message below</p>
                    </div>
                ) : (
                    <div className="space-y-4">
                        {messages.map((message, index) => (
                            <div
                                key={index}
                                className={`flex ${message.role === 'user' ? 'justify-end' : 'justify-start'}`}
                            >
                                <div
                                    className={`max-w-[70%] rounded-lg p-3 ${
                                        message.role === 'user'
                                            ? 'bg-blue-500 text-white'
                                            : 'bg-white border border-gray-200'
                                    }`}
                                >
                                    <div className="text-xs text-gray-500 mb-1">
                                        {message.role === 'user' ? 'You' : 'AI'} • {formatTimestamp()}
                                    </div>
                                    <div className="whitespace-pre-wrap">{message.content}</div>
                                </div>
                            </div>
                        ))}
                        {/* Streaming Response */}
                        {isStreaming && chunks.length > 0 && (
                            <div className="flex justify-start">
                                <div className="max-w-[70%] rounded-lg p-3 bg-white border border-gray-200">
                                    <div className="text-xs text-gray-500 mb-1">
                                        AI • Streaming...
                                    </div>
                                    <div className="whitespace-pre-wrap">
                                        {fullText}
                                        <span className="inline-block w-2 h-5 bg-blue-500 animate-pulse ml-1" />
                                    </div>
                                </div>
                            </div>
                        )}
                        <div ref={messagesEndRef} />
                    </div>
                )}
            </div>

            {/* Input Area */}
            <div className="flex gap-2">
                <input
                    type="text"
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyPress={(e) => {
                        if (e.key === 'Enter' && !e.shiftKey) {
                            e.preventDefault();
                            handleSend();
                        }
                    }}
                    placeholder="Type your message... (Press Enter to send)"
                    className="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                    disabled={isStreaming}
                />
                <button
                    onClick={handleSend}
                    disabled={!input.trim() || isStreaming}
                    className="px-6 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50"
                >
                    {isStreaming ? 'Streaming...' : 'Send'}
                </button>
            </div>

            {/* Streaming Info */}
            {isStreaming && (
                <div className="mt-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
                    <div className="flex items-center gap-2 mb-2">
                        <div className="w-3 h-3 bg-blue-500 rounded-full animate-pulse" />
                        <span className="font-medium text-blue-700">Streaming in progress...</span>
                    </div>
                    <div className="text-sm text-gray-600">
                        <div>Chunks received: {chunks.length}</div>
                        <div>Current length: {fullText.length} characters</div>
                    </div>
                </div>
            )}

            {/* Chunk Preview */}
            {chunks.length > 0 && !isStreaming && (
                <div className="mt-4 p-4 bg-green-50 border border-green-200 rounded-lg">
                    <h3 className="font-semibold text-green-800 mb-2">Streaming Complete</h3>
                    <div className="text-sm text-gray-600">
                        <div>Total chunks: {chunks.length}</div>
                        <div>Final length: {fullText.length} characters</div>
                    </div>
                </div>
            )}
        </div>
    );
}

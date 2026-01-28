// Rainy Cowork - useStreaming Hook (PHASE 3)
// React hook for streaming chat completions using Phase 3 commands

import { useCallback, useState, useRef } from 'react';
import * as tauri from '../services/tauri';
import type { ChatCompletionRequestDto, StreamingChunk } from '../services/tauri';

interface UseStreamingResult {
    isStreaming: boolean;
    error: string | null;
    chunks: StreamingChunk[];
    fullText: string;
    streamChat: (request: ChatCompletionRequestDto, onChunk?: (chunk: StreamingChunk) => void) => Promise<void>;
    stopStreaming: () => void;
    resetStream: () => void;
}

export function useStreaming(): UseStreamingResult {
    const [isStreaming, setIsStreaming] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [chunks, setChunks] = useState<StreamingChunk[]>([]);
    const abortControllerRef = useRef<AbortController | null>(null);

    // Calculate full text from chunks
    const fullText = chunks
        .filter(chunk => !chunk.is_final)
        .map(chunk => chunk.content)
        .join('');

    const streamChat = useCallback(async (
        request: ChatCompletionRequestDto,
        onChunk?: (chunk: StreamingChunk) => void
    ) => {
        // Reset state
        setIsStreaming(true);
        setError(null);
        setChunks([]);

        // Create abort controller for cancellation
        abortControllerRef.current = new AbortController();

        try {
            // Note: The current Phase 3 implementation doesn't support streaming yet
            // This is a placeholder for future implementation
            // For now, we'll use the non-streaming complete_chat command

            const response = await tauri.completeChat({
                ...request,
                stream: false, // Force non-streaming for now
            });

            // Simulate streaming by breaking the response into chunks
            const content = response.content;
            const chunkSize = 10; // Characters per chunk
            const totalChunks = Math.ceil(content.length / chunkSize);

            for (let i = 0; i < totalChunks; i++) {
                // Check if streaming was cancelled
                if (abortControllerRef.current?.signal.aborted) {
                    break;
                }

                const start = i * chunkSize;
                const end = Math.min(start + chunkSize, content.length);
                const chunkContent = content.slice(start, end);

                const chunk: StreamingChunk = {
                    content: chunkContent,
                    is_final: i === totalChunks - 1,
                    finish_reason: i === totalChunks - 1 ? response.finish_reason : undefined,
                };

                setChunks(prev => [...prev, chunk]);

                // Call the onChunk callback if provided
                if (onChunk) {
                    onChunk(chunk);
                }

                // Small delay to simulate streaming
                await new Promise(resolve => setTimeout(resolve, 50));
            }

        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        } finally {
            setIsStreaming(false);
            abortControllerRef.current = null;
        }
    }, []);

    const stopStreaming = useCallback(() => {
        if (abortControllerRef.current) {
            abortControllerRef.current.abort();
            abortControllerRef.current = null;
        }
        setIsStreaming(false);
    }, []);

    const resetStream = useCallback(() => {
        setIsStreaming(false);
        setError(null);
        setChunks([]);
        if (abortControllerRef.current) {
            abortControllerRef.current.abort();
            abortControllerRef.current = null;
        }
    }, []);

    return {
        isStreaming,
        error,
        chunks,
        fullText,
        streamChat,
        stopStreaming,
        resetStream,
    };
}

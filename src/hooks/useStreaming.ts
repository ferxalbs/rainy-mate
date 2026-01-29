// Rainy Cowork - useStreaming Hook (PHASE 3)
// React hook for streaming chat completions using intelligent routing

import { useCallback, useState, useRef } from 'react';
import * as tauri from '../services/tauri';
import type { 
    ChatCompletionRequestDto, 
    StreamingChunk, 
    StreamingEvent,
    RoutedChatRequest 
} from '../services/tauri';

interface UseStreamingResult {
    isStreaming: boolean;
    error: string | null;
    chunks: StreamingChunk[];
    fullText: string;
    model: string | null;
    providerId: string | null;
    finishReason: string | null;
    totalChunks: number;
    
    // Streaming methods
    streamChat: (request: ChatCompletionRequestDto, onChunk?: (chunk: StreamingChunk) => void) => Promise<void>;
    streamWithRouting: (request: RoutedChatRequest, onEvent?: (event: StreamingEvent) => void) => Promise<void>;
    stopStreaming: () => void;
    resetStream: () => void;
}

export function useStreaming(): UseStreamingResult {
    const [isStreaming, setIsStreaming] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [chunks, setChunks] = useState<StreamingChunk[]>([]);
    const [model, setModel] = useState<string | null>(null);
    const [providerId, setProviderId] = useState<string | null>(null);
    const [finishReason, setFinishReason] = useState<string | null>(null);
    const [totalChunks, setTotalChunks] = useState(0);
    const abortRef = useRef(false);

    // Calculate full text from chunks
    const fullText = chunks
        .filter(chunk => !chunk.is_final)
        .map(chunk => chunk.content)
        .join('');

    /**
     * Stream chat using the intelligent router
     * This is the primary streaming method that uses PHASE 3 routing
     */
    const streamWithRouting = useCallback(async (
        request: RoutedChatRequest,
        onEvent?: (event: StreamingEvent) => void
    ) => {
        // Reset state
        setIsStreaming(true);
        setError(null);
        setChunks([]);
        setModel(null);
        setProviderId(null);
        setFinishReason(null);
        setTotalChunks(0);
        abortRef.current = false;

        try {
            await tauri.streamWithRouting(request, (event) => {
                // Check if aborted
                if (abortRef.current) {
                    return;
                }

                switch (event.event) {
                    case 'started':
                        setModel(event.data.model);
                        setProviderId(event.data.providerId);
                        break;
                    
                    case 'chunk': {
                        const chunk: StreamingChunk = {
                            content: event.data.content,
                            is_final: event.data.isFinal,
                        };
                        setChunks(prev => [...prev, chunk]);
                        break;
                    }
                    
                    case 'finished':
                        setFinishReason(event.data.finishReason);
                        setTotalChunks(event.data.totalChunks);
                        setIsStreaming(false);
                        break;
                    
                    case 'error':
                        setError(event.data.message);
                        setIsStreaming(false);
                        break;
                }

                // Forward event to caller
                if (onEvent) {
                    onEvent(event);
                }
            });
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            setIsStreaming(false);
            throw new Error(message);
        }
    }, []);

    /**
     * Legacy streaming method for backward compatibility
     * Converts ChatCompletionRequestDto to RoutedChatRequest and uses router
     */
    const streamChat = useCallback(async (
        request: ChatCompletionRequestDto,
        onChunk?: (chunk: StreamingChunk) => void
    ) => {
        // Convert to routed request format
        const routedRequest: RoutedChatRequest = {
            messages: request.messages,
            model: request.model,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            preferred_provider: request.provider_id,
        };

        // Use router streaming
        await streamWithRouting(routedRequest, (event) => {
            // Convert to chunk callback for backward compatibility
            if (event.event === 'chunk' && onChunk) {
                const chunk: StreamingChunk = {
                    content: event.data.content,
                    is_final: event.data.isFinal,
                };
                onChunk(chunk);
            }
        });
    }, [streamWithRouting]);

    const stopStreaming = useCallback(() => {
        abortRef.current = true;
        setIsStreaming(false);
    }, []);

    const resetStream = useCallback(() => {
        abortRef.current = false;
        setIsStreaming(false);
        setError(null);
        setChunks([]);
        setModel(null);
        setProviderId(null);
        setFinishReason(null);
        setTotalChunks(0);
    }, []);

    return {
        isStreaming,
        error,
        chunks,
        fullText,
        model,
        providerId,
        finishReason,
        totalChunks,
        streamChat,
        streamWithRouting,
        stopStreaming,
        resetStream,
    };
}

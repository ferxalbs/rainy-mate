// Rainy Cowork - useIntelligentRouter Hook (PHASE 3)
// React hook for intelligent AI routing with load balancing, fallback, and streaming

import { useCallback, useEffect, useState, useRef } from "react";
import * as tauri from "../services/tauri";
import type {
  RouterConfigDto,
  RouterStatsDto,
  RoutedChatRequest,
  RoutedEmbeddingRequest,
  ChatCompletionResponse,
  EmbeddingResponse,
  StreamingEvent,
} from "../services/tauri";

interface UseIntelligentRouterResult {
  // Configuration
  config: RouterConfigDto | null;
  stats: RouterStatsDto | null;
  providers: string[];
  hasProviders: boolean;
  isLoading: boolean;
  error: string | null;

  // Configuration management
  refreshConfig: () => Promise<void>;
  updateConfig: (config: Partial<RouterConfigDto>) => Promise<RouterConfigDto>;

  // Statistics
  refreshStats: () => Promise<void>;

  // Provider management
  refreshProviders: () => Promise<void>;
  addProvider: (providerId: string) => Promise<void>;
  removeProvider: (providerId: string) => Promise<void>;

  // Completions
  completeWithRouting: (
    request: RoutedChatRequest,
  ) => Promise<ChatCompletionResponse>;
  streamWithRouting: (
    request: RoutedChatRequest,
    onEvent: (event: StreamingEvent) => void,
  ) => Promise<void>;
  embedWithRouting: (
    request: RoutedEmbeddingRequest,
  ) => Promise<EmbeddingResponse>;

  // Streaming state
  isStreaming: boolean;
  streamingContent: string;
  stopStreaming: () => void;
  resetStreaming: () => void;
}

export function useIntelligentRouter(): UseIntelligentRouterResult {
  const [config, setConfig] = useState<RouterConfigDto | null>(null);
  const [stats, setStats] = useState<RouterStatsDto | null>(null);
  const [providers, setProviders] = useState<string[]>([]);
  const [hasProviders, setHasProviders] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Streaming state
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamingContent, setStreamingContent] = useState("");
  const abortStreamRef = useRef(false);

  // Auto-sync registered providers to router on init
  useEffect(() => {
    const initRouter = async () => {
      setIsLoading(true);
      try {
        // Get all registered providers from the registry
        const allProviders = await tauri.listAllProviders();

        // Get current router providers
        const routerProviders = await tauri.getRouterProviders();
        const routerProviderSet = new Set(routerProviders);

        // Add any registered providers not in the router
        for (const provider of allProviders) {
          if (provider.enabled && !routerProviderSet.has(provider.id)) {
            try {
              await tauri.addProviderToRouter(provider.id);
              console.log(`[Router] Auto-added provider: ${provider.id}`);
            } catch (err) {
              console.warn(
                `[Router] Failed to add provider ${provider.id}:`,
                err,
              );
            }
          }
        }

        // Now refresh all router state
        await refreshConfig();
        await refreshStats();
        await refreshProviders();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
      } finally {
        setIsLoading(false);
      }
    };

    initRouter();
  }, []);

  const refreshConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const cfg = await tauri.getRouterConfig();
      setConfig(cfg);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const updateConfig = useCallback(
    async (newConfig: Partial<RouterConfigDto>): Promise<RouterConfigDto> => {
      setError(null);
      try {
        const updated = await tauri.updateRouterConfig(newConfig);
        setConfig(updated);
        return updated;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw new Error(message);
      }
    },
    [],
  );

  const refreshStats = useCallback(async () => {
    setError(null);
    try {
      const s = await tauri.getRouterStats();
      setStats(s);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    }
  }, []);

  const refreshProviders = useCallback(async () => {
    setError(null);
    try {
      const [providerList, hasAny] = await Promise.all([
        tauri.getRouterProviders(),
        tauri.routerHasProviders(),
      ]);
      setProviders(providerList);
      setHasProviders(hasAny);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    }
  }, []);

  const addProvider = useCallback(
    async (providerId: string) => {
      setError(null);
      try {
        await tauri.addProviderToRouter(providerId);
        await refreshProviders();
        await refreshStats();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw new Error(message);
      }
    },
    [refreshProviders, refreshStats],
  );

  const removeProvider = useCallback(
    async (providerId: string) => {
      setError(null);
      try {
        await tauri.removeProviderFromRouter(providerId);
        await refreshProviders();
        await refreshStats();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw new Error(message);
      }
    },
    [refreshProviders, refreshStats],
  );

  const completeWithRouting = useCallback(
    async (request: RoutedChatRequest): Promise<ChatCompletionResponse> => {
      setError(null);
      try {
        return await tauri.completeWithRouting(request);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw new Error(message);
      }
    },
    [],
  );

  const streamWithRouting = useCallback(
    async (
      request: RoutedChatRequest,
      onEvent: (event: StreamingEvent) => void,
    ): Promise<void> => {
      setError(null);
      setIsStreaming(true);
      setStreamingContent("");
      abortStreamRef.current = false;

      try {
        await tauri.streamWithRouting(request, (event) => {
          // Check if aborted
          if (abortStreamRef.current) {
            return;
          }

          // Update streaming content for chunk events
          if (event.event === "chunk") {
            setStreamingContent((prev) => prev + event.data.content);
          }

          // Forward event to caller
          onEvent(event);

          // Handle completion
          if (event.event === "finished" || event.event === "error") {
            setIsStreaming(false);
          }
        });
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        setIsStreaming(false);
        throw new Error(message);
      }
    },
    [],
  );

  const embedWithRouting = useCallback(
    async (request: RoutedEmbeddingRequest): Promise<EmbeddingResponse> => {
      setError(null);
      try {
        return await tauri.embedWithRouting(request);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        throw new Error(message);
      }
    },
    [],
  );

  const stopStreaming = useCallback(() => {
    abortStreamRef.current = true;
    setIsStreaming(false);
  }, []);

  const resetStreaming = useCallback(() => {
    abortStreamRef.current = false;
    setIsStreaming(false);
    setStreamingContent("");
  }, []);

  return {
    config,
    stats,
    providers,
    hasProviders,
    isLoading,
    error,
    refreshConfig,
    updateConfig,
    refreshStats,
    refreshProviders,
    addProvider,
    removeProvider,
    completeWithRouting,
    streamWithRouting,
    embedWithRouting,
    isStreaming,
    streamingContent,
    stopStreaming,
    resetStreaming,
  };
}

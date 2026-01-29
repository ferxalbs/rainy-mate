// Rainy Cowork - useAIProvider Hook (PHASE 3)
// React hook for AI provider management using new Phase 3 commands

import { useCallback, useEffect, useState, useRef } from 'react';
import * as tauri from '../services/tauri';
import type {
    ProviderInfo,
    ProviderStatsDto,
    RegisterProviderRequest,
    ChatCompletionRequestDto,
    ChatCompletionResponse,
    EmbeddingRequestDto,
    EmbeddingResponse,
} from '../services/tauri';

interface UseAIProviderResult {
    providers: ProviderInfo[];
    defaultProvider: ProviderInfo | null;
    isLoading: boolean;
    error: string | null;
    providerCount: number;
    refreshProviders: () => Promise<void>;
    getProviderInfo: (id: string) => Promise<ProviderInfo>;
    registerProvider: (request: RegisterProviderRequest) => Promise<string>;
    unregisterProvider: (id: string) => Promise<void>;
    setDefaultProvider: (id: string) => Promise<void>;
    getProviderStats: (id: string) => Promise<ProviderStatsDto>;
    getAllProviderStats: () => Promise<[string, ProviderStatsDto][]>;
    testProviderConnection: (id: string) => Promise<string>;
    getProviderCapabilities: (id: string) => Promise<tauri.ProviderCapabilities>;
    completeChat: (request: ChatCompletionRequestDto) => Promise<ChatCompletionResponse>;
    generateEmbeddings: (request: EmbeddingRequestDto) => Promise<EmbeddingResponse>;
    getProviderAvailableModels: (id: string) => Promise<string[]>;
    clearProviders: () => Promise<void>;

    // Legacy/Compatible API Key Methods
    hasApiKey: (provider: string) => boolean; // Synchronous check from cache
    validateApiKey: (provider: string, apiKey: string) => Promise<boolean>;
    storeApiKey: (provider: string, apiKey: string) => Promise<void>;
    getApiKey: (provider: string) => Promise<string | null>;
    deleteApiKey: (provider: string) => Promise<void>;
}

export function useAIProvider(): UseAIProviderResult {
    const [providers, setProviders] = useState<ProviderInfo[]>([]);
    const [defaultProvider, setDefaultProviderState] = useState<ProviderInfo | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [providerCount, setProviderCount] = useState(0);
    const [apiKeyStatus, setApiKeyStatus] = useState<Record<string, boolean>>({});

    // Debounce refresh calls to prevent excessive API calls
    const refreshTimeoutRef = useRef<NodeJS.Timeout | null>(null);
    const lastRefreshTime = useRef<number>(0);

    const refreshProviders = useCallback(async (force = false) => {
        // Debounce rapid calls (unless forced)
        const now = Date.now();
        if (!force && now - lastRefreshTime.current < 1000) {
            return;
        }
        lastRefreshTime.current = now;

        // Clear any pending timeout
        if (refreshTimeoutRef.current) {
            clearTimeout(refreshTimeoutRef.current);
        }

        setIsLoading(true);
        setError(null);
        try {
            const providerList = await tauri.listAllProviders();
            setProviders(providerList);
            setProviderCount(providerList.length);

            // Get default provider
            try {
                const defaultProv = await tauri.getDefaultProvider();
                setDefaultProviderState(defaultProv);
            } catch (err) {
                setDefaultProviderState(null);
            }

            // Check API keys for common providers
            // This is needed for legacy/sync compatibility
            const keyStatus: Record<string, boolean> = {};
            const commonProviders = ['rainy_api', 'cowork_api', 'gemini', 'openai', 'anthropic'];
            
            // Also check items from the list
            const providerIds = new Set([...commonProviders, ...providerList.map(p => p.id)]);
            
            await Promise.all(Array.from(providerIds).map(async (pid) => {
                try {
                    const has = await tauri.hasApiKey(pid);
                    keyStatus[pid] = has;
                } catch (e) {
                    keyStatus[pid] = false;
                }
            }));
            
            setApiKeyStatus(keyStatus);

        } catch (err) {
            setError(err instanceof Error ? err.message : String(err));
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        refreshProviders();
    }, [refreshProviders]);

    const getProviderInfo = useCallback(async (id: string): Promise<ProviderInfo> => {
        setError(null);
        try {
            return await tauri.getProviderInfo(id);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const registerProvider = useCallback(async (
        request: RegisterProviderRequest
    ): Promise<string> => {
        setError(null);
        try {
            const id = await tauri.registerProvider(request);
            // Force refresh after registration
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
            return id;
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, [refreshProviders]);

    const unregisterProvider = useCallback(async (id: string) => {
        setError(null);
        try {
            await tauri.unregisterProvider(id);
            // Force refresh after unregistration
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, [refreshProviders]);

    const setDefaultProvider = useCallback(async (id: string) => {
        setError(null);
        try {
            await tauri.setDefaultProvider(id);
            // Force refresh after setting default
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, [refreshProviders]);

    const getProviderStats = useCallback(async (id: string): Promise<ProviderStatsDto> => {
        setError(null);
        try {
            return await tauri.getProviderStats(id);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const getAllProviderStats = useCallback(async (): Promise<[string, ProviderStatsDto][]> => {
        setError(null);
        try {
            return await tauri.getAllProviderStats();
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const testProviderConnection = useCallback(async (id: string): Promise<string> => {
        setError(null);
        try {
            return await tauri.testProviderConnection(id);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const getProviderCapabilities = useCallback(async (id: string): Promise<tauri.ProviderCapabilities> => {
        setError(null);
        try {
            return await tauri.getProviderCapabilities(id);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const completeChat = useCallback(async (
        request: ChatCompletionRequestDto
    ): Promise<ChatCompletionResponse> => {
        setError(null);
        try {
            return await tauri.completeChat(request);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const generateEmbeddings = useCallback(async (
        request: EmbeddingRequestDto
    ): Promise<EmbeddingResponse> => {
        setError(null);
        try {
            return await tauri.generateEmbeddings(request);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const getProviderAvailableModels = useCallback(async (id: string): Promise<string[]> => {
        setError(null);
        try {
            return await tauri.getProviderAvailableModels(id);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, []);

    const clearProviders = useCallback(async () => {
        setError(null);
        try {
            await tauri.clearProviders();
            // Force refresh after clearing
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
            throw new Error(message);
        }
    }, [refreshProviders]);

    // Legacy/Compatible API Key Methods
    const hasApiKey = useCallback((provider: string): boolean => {
        return !!apiKeyStatus[provider];
    }, [apiKeyStatus]);

    const validateApiKey = useCallback(async (provider: string, apiKey: string): Promise<boolean> => {
        try {
            const isValid = await tauri.validateApiKey(provider, apiKey);
            if (isValid) {
                 // Optimistically update status
                 setApiKeyStatus(prev => ({ ...prev, [provider]: true }));
            }
            return isValid;
        } catch (err) {
            console.error(`Error validating API key for ${provider}:`, err);
            return false;
        }
    }, []);

    const storeApiKey = useCallback(async (provider: string, apiKey: string): Promise<void> => {
        try {
            await tauri.storeApiKey(provider, apiKey);
            // Optimistically update status
            setApiKeyStatus(prev => ({ ...prev, [provider]: true }));
            // Force refresh
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            throw new Error(message);
        }
    }, [refreshProviders]);

    const getApiKey = useCallback(async (provider: string): Promise<string | null> => {
        try {
            return await tauri.getApiKey(provider);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            throw new Error(message);
        }
    }, []);

    const deleteApiKey = useCallback(async (provider: string): Promise<void> => {
        try {
            await tauri.deleteApiKey(provider);
            // Optimistically update status
            setApiKeyStatus(prev => ({ ...prev, [provider]: false }));
            // Force refresh
            refreshTimeoutRef.current = setTimeout(() => {
                refreshProviders(true);
            }, 500);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            throw new Error(message);
        }
    }, [refreshProviders]);

    return {
        providers,
        defaultProvider,
        isLoading,
        error,
        providerCount,
        refreshProviders,
        getProviderInfo,
        registerProvider,
        unregisterProvider,
        setDefaultProvider,
        getProviderStats,
        getAllProviderStats,
        testProviderConnection,
        getProviderCapabilities,
        completeChat,
        generateEmbeddings,
        getProviderAvailableModels,
        clearProviders,
        hasApiKey,
        validateApiKey,
        storeApiKey,
        getApiKey,
        deleteApiKey,
    };
}

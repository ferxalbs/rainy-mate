// Rainy MaTE - useBatchOperations Hook
// Optimized batch API operations to reduce connection overhead

import { useCallback, useRef } from 'react';
import * as tauri from '../services/tauri';

interface BatchValidationResult {
    provider: string;
    isValid: boolean;
    error?: string;
}

interface UseBatchOperationsResult {
    batchValidateKeys: (providers: Array<{ provider: string; apiKey: string }>) => Promise<BatchValidationResult[]>;
    batchGetModels: (providers: string[]) => Promise<Record<string, string[]>>;
    debouncedRefresh: (fn: () => Promise<void>, delay?: number) => void;
}

export function useBatchOperations(): UseBatchOperationsResult {
    const debounceTimeoutRef = useRef<NodeJS.Timeout | null>(null);

    const batchValidateKeys = useCallback(async (
        providers: Array<{ provider: string; apiKey: string }>
    ): Promise<BatchValidationResult[]> => {
        const validationPromises = providers.map(async ({ provider, apiKey }) => {
            try {
                const isValid = await tauri.validateApiKey(provider, apiKey);
                return { provider, isValid };
            } catch (error) {
                return { 
                    provider, 
                    isValid: false, 
                    error: error instanceof Error ? error.message : String(error)
                };
            }
        });

        return Promise.all(validationPromises);
    }, []);

    const batchGetModels = useCallback(async (providers: string[]): Promise<Record<string, string[]>> => {
        const modelPromises = providers.map(async (provider) => {
            try {
                const models = await tauri.getProviderModels(provider);
                return { provider, models };
            } catch (error) {
                console.warn(`Failed to get models for ${provider}:`, error);
                return { provider, models: [] };
            }
        });

        const results = await Promise.allSettled(modelPromises);
        const modelMap: Record<string, string[]> = {};

        results.forEach((result, index) => {
            const provider = providers[index];
            if (result.status === 'fulfilled') {
                modelMap[provider] = result.value.models;
            } else {
                modelMap[provider] = [];
            }
        });

        return modelMap;
    }, []);

    const debouncedRefresh = useCallback((fn: () => Promise<void>, delay = 300) => {
        if (debounceTimeoutRef.current) {
            clearTimeout(debounceTimeoutRef.current);
        }

        debounceTimeoutRef.current = setTimeout(async () => {
            try {
                await fn();
            } catch (error) {
                console.error('Debounced refresh failed:', error);
            }
        }, delay);
    }, []);

    return {
        batchValidateKeys,
        batchGetModels,
        debouncedRefresh,
    };
}
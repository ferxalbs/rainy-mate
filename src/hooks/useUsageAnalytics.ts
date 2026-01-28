// Rainy Cowork - useUsageAnalytics Hook (PHASE 3)
// React hook for usage analytics tracking using Phase 3 commands

import { useCallback, useEffect, useState } from 'react';
import * as tauri from '../services/tauri';
import type { ProviderStatsDto } from '../services/tauri';

interface UseUsageAnalyticsResult {
    stats: Map<string, ProviderStatsDto>;
    isLoading: boolean;
    error: string | null;
    totalRequests: number;
    totalTokens: number;
    totalSuccessfulRequests: number;
    totalFailedRequests: number;
    averageLatency: number;
    refreshStats: () => Promise<void>;
    getProviderStats: (id: string) => Promise<ProviderStatsDto>;
    getStatsForProvider: (id: string) => ProviderStatsDto | undefined;
    getTopProvidersByRequests: (limit?: number) => Array<{ id: string; stats: ProviderStatsDto }>;
    getTopProvidersByTokens: (limit?: number) => Array<{ id: string; stats: ProviderStatsDto }>;
    getMostReliableProviders: (limit?: number) => Array<{ id: string; stats: ProviderStatsDto; reliability: number }>;
}

export function useUsageAnalytics(): UseUsageAnalyticsResult {
    const [stats, setStats] = useState<Map<string, ProviderStatsDto>>(new Map());
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const refreshStats = useCallback(async () => {
        setIsLoading(true);
        setError(null);
        try {
            const allStats = await tauri.getAllProviderStats();
            const statsMap = new Map(allStats);
            setStats(statsMap);
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err);
            setError(message);
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        refreshStats();
    }, [refreshStats]);

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

    const getStatsForProvider = useCallback((id: string): ProviderStatsDto | undefined => {
        return stats.get(id);
    }, [stats]);

    const getTopProvidersByRequests = useCallback((limit = 5) => {
        return Array.from(stats.entries())
            .map(([id, stat]) => ({ id, stats: stat }))
            .sort((a, b) => b.stats.total_requests - a.stats.total_requests)
            .slice(0, limit);
    }, [stats]);

    const getTopProvidersByTokens = useCallback((limit = 5) => {
        return Array.from(stats.entries())
            .map(([id, stat]) => ({ id, stats: stat }))
            .sort((a, b) => b.stats.total_tokens - a.stats.total_tokens)
            .slice(0, limit);
    }, [stats]);

    const getMostReliableProviders = useCallback((limit = 5) => {
        return Array.from(stats.entries())
            .map(([id, stat]) => {
                const reliability = stat.total_requests > 0
                    ? (stat.successful_requests / stat.total_requests) * 100
                    : 0;
                return { id, stats: stat, reliability };
            })
            .sort((a, b) => b.reliability - a.reliability)
            .slice(0, limit);
    }, [stats]);

    // Calculate aggregate statistics
    const totalRequests = Array.from(stats.values()).reduce((sum, stat) => sum + stat.total_requests, 0);
    const totalTokens = Array.from(stats.values()).reduce((sum, stat) => sum + stat.total_tokens, 0);
    const totalSuccessfulRequests = Array.from(stats.values()).reduce((sum, stat) => sum + stat.successful_requests, 0);
    const totalFailedRequests = Array.from(stats.values()).reduce((sum, stat) => sum + stat.failed_requests, 0);
    const averageLatency = Array.from(stats.values()).reduce((sum, stat) => sum + stat.avg_latency_ms, 0) / (stats.size || 1);

    return {
        stats,
        isLoading,
        error,
        totalRequests,
        totalTokens,
        totalSuccessfulRequests,
        totalFailedRequests,
        averageLatency,
        refreshStats,
        getProviderStats,
        getStatsForProvider,
        getTopProvidersByRequests,
        getTopProvidersByTokens,
        getMostReliableProviders,
    };
}

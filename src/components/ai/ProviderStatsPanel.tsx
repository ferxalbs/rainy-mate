// Rainy Cowork - Provider Stats Panel (PHASE 3)
// UI component for displaying AI provider usage analytics

import React from 'react';
import { useUsageAnalytics } from '../../hooks/useUsageAnalytics';

export function ProviderStatsPanel() {
    const {
        stats,
        isLoading,
        error,
        totalRequests,
        totalTokens,
        totalSuccessfulRequests,
        totalFailedRequests,
        averageLatency,
        refreshStats,
        getTopProvidersByRequests,
        getTopProvidersByTokens,
        getMostReliableProviders,
    } = useUsageAnalytics();

    const topByRequests = getTopProvidersByRequests(5);
    const topByTokens = getTopProvidersByTokens(5);
    const mostReliable = getMostReliableProviders(5);

    const formatNumber = (num: number) => {
        if (num >= 1000000) {
            return `${(num / 1000000).toFixed(1)}M`;
        } else if (num >= 1000) {
            return `${(num / 1000).toFixed(1)}K`;
        }
        return num.toString();
    };

    const formatLatency = (ms: number) => {
        if (ms >= 1000) {
            return `${(ms / 1000).toFixed(2)}s`;
        }
        return `${ms.toFixed(0)}ms`;
    };

    const getReliabilityColor = (reliability: number) => {
        if (reliability >= 95) return 'text-green-500';
        if (reliability >= 80) return 'text-yellow-500';
        return 'text-red-500';
    };

    return (
        <div className="backdrop-blur-md bg-white/80 rounded-xl shadow-lg p-6">
            <div className="flex items-center justify-between mb-6">
                <h2 className="text-2xl font-bold">Provider Usage Analytics</h2>
                <button
                    onClick={() => refreshStats()}
                    disabled={isLoading}
                    className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50"
                >
                    {isLoading ? 'Refreshing...' : 'Refresh'}
                </button>
            </div>

            {error && (
                <div className="mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg">
                    {error}
                </div>
            )}

            {/* Overall Statistics */}
            <div className="mb-6">
                <h3 className="text-lg font-semibold mb-4">Overall Statistics</h3>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                    <div className="p-4 bg-blue-50 rounded-lg border border-blue-200">
                        <div className="text-sm text-gray-600 mb-1">Total Requests</div>
                        <div className="text-2xl font-bold text-blue-600">{formatNumber(totalRequests)}</div>
                    </div>
                    <div className="p-4 bg-green-50 rounded-lg border border-green-200">
                        <div className="text-sm text-gray-600 mb-1">Total Tokens</div>
                        <div className="text-2xl font-bold text-green-600">{formatNumber(totalTokens)}</div>
                    </div>
                    <div className="p-4 bg-purple-50 rounded-lg border border-purple-200">
                        <div className="text-sm text-gray-600 mb-1">Success Rate</div>
                        <div className="text-2xl font-bold text-purple-600">
                            {totalRequests > 0
                                ? `${((totalSuccessfulRequests / totalRequests) * 100).toFixed(1)}%`
                                : '0%'}
                        </div>
                    </div>
                    <div className="p-4 bg-orange-50 rounded-lg border border-orange-200">
                        <div className="text-sm text-gray-600 mb-1">Avg Latency</div>
                        <div className="text-2xl font-bold text-orange-600">{formatLatency(averageLatency)}</div>
                    </div>
                </div>
            </div>

            {/* Request Breakdown */}
            <div className="mb-6">
                <h3 className="text-lg font-semibold mb-4">Request Breakdown</h3>
                <div className="grid grid-cols-2 gap-4">
                    <div className="p-4 bg-green-50 rounded-lg border border-green-200">
                        <div className="text-sm text-gray-600 mb-1">Successful</div>
                        <div className="text-xl font-bold text-green-600">{formatNumber(totalSuccessfulRequests)}</div>
                    </div>
                    <div className="p-4 bg-red-50 rounded-lg border border-red-200">
                        <div className="text-sm text-gray-600 mb-1">Failed</div>
                        <div className="text-xl font-bold text-red-600">{formatNumber(totalFailedRequests)}</div>
                    </div>
                </div>
            </div>

            {/* Top Providers by Requests */}
            <div className="mb-6">
                <h3 className="text-lg font-semibold mb-4">Top Providers by Requests</h3>
                {topByRequests.length === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                        No data available
                    </div>
                ) : (
                    <div className="space-y-2">
                        {topByRequests.map(({ id, stats }) => (
                            <div
                                key={id}
                                className="p-4 bg-white rounded-lg border border-gray-200"
                            >
                                <div className="flex items-center justify-between">
                                    <div>
                                        <div className="font-semibold text-lg">{id}</div>
                                        <div className="text-sm text-gray-600">
                                            {stats.total_requests} request{stats.total_requests !== 1 ? 's' : ''}
                                        </div>
                                    </div>
                                    <div className="text-right">
                                        <div className="text-sm text-gray-600">Tokens</div>
                                        <div className="font-bold text-blue-600">{formatNumber(stats.total_tokens)}</div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Top Providers by Tokens */}
            <div className="mb-6">
                <h3 className="text-lg font-semibold mb-4">Top Providers by Tokens</h3>
                {topByTokens.length === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                        No data available
                    </div>
                ) : (
                    <div className="space-y-2">
                        {topByTokens.map(({ id, stats }) => (
                            <div
                                key={id}
                                className="p-4 bg-white rounded-lg border border-gray-200"
                            >
                                <div className="flex items-center justify-between">
                                    <div>
                                        <div className="font-semibold text-lg">{id}</div>
                                        <div className="text-sm text-gray-600">
                                            {stats.total_requests} request{stats.total_requests !== 1 ? 's' : ''}
                                        </div>
                                    </div>
                                    <div className="text-right">
                                        <div className="text-sm text-gray-600">Tokens</div>
                                        <div className="font-bold text-green-600">{formatNumber(stats.total_tokens)}</div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Most Reliable Providers */}
            <div className="mb-6">
                <h3 className="text-lg font-semibold mb-4">Most Reliable Providers</h3>
                {mostReliable.length === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                        No data available
                    </div>
                ) : (
                    <div className="space-y-2">
                        {mostReliable.map(({ id, stats, reliability }) => (
                            <div
                                key={id}
                                className="p-4 bg-white rounded-lg border border-gray-200"
                            >
                                <div className="flex items-center justify-between">
                                    <div>
                                        <div className="font-semibold text-lg">{id}</div>
                                        <div className="text-sm text-gray-600">
                                            {stats.total_requests} request{stats.total_requests !== 1 ? 's' : ''}
                                        </div>
                                    </div>
                                    <div className="text-right">
                                        <div className="text-sm text-gray-600">Reliability</div>
                                        <div className={`font-bold ${getReliabilityColor(reliability)}`}>
                                            {reliability.toFixed(1)}%
                                        </div>
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Detailed Stats Table */}
            <div>
                <h3 className="text-lg font-semibold mb-4">Detailed Statistics</h3>
                {stats.size === 0 ? (
                    <div className="text-center py-8 text-gray-500">
                        No provider statistics available
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full border-collapse">
                            <thead>
                                <tr className="bg-gray-100">
                                    <th className="px-4 py-2 text-left border border-gray-300">Provider</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Requests</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Successful</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Failed</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Tokens</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Avg Latency</th>
                                    <th className="px-4 py-2 text-right border border-gray-300">Success Rate</th>
                                </tr>
                            </thead>
                            <tbody>
                                {Array.from(stats.entries()).map(([id, stat]) => (
                                    <tr key={id} className="border-b border-gray-200">
                                        <td className="px-4 py-2 font-medium">{id}</td>
                                        <td className="px-4 py-2 text-right">{formatNumber(stat.total_requests)}</td>
                                        <td className="px-4 py-2 text-right text-green-600">
                                            {formatNumber(stat.successful_requests)}
                                        </td>
                                        <td className="px-4 py-2 text-right text-red-600">
                                            {formatNumber(stat.failed_requests)}
                                        </td>
                                        <td className="px-4 py-2 text-right">{formatNumber(stat.total_tokens)}</td>
                                        <td className="px-4 py-2 text-right">{formatLatency(stat.avg_latency_ms)}</td>
                                        <td className="px-4 py-2 text-right">
                                            {stat.total_requests > 0
                                                ? `${((stat.successful_requests / stat.total_requests) * 100).toFixed(1)}%`
                                                : '0%'}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>
        </div>
    );
}

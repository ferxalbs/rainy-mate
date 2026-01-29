// Rainy Cowork - Router Config Panel (PHASE 3)
// UI component for configuring the Intelligent Router

import { useState, useEffect } from "react";
import { useIntelligentRouter } from "../../hooks/useIntelligentRouter";
import { useAIProvider } from "../../hooks/useAIProvider";

export function RouterConfigPanel() {
  const {
    config,
    stats,
    providers: routerProviders,
    isLoading,
    error,
    refreshConfig,
    updateConfig,
    refreshStats,
    refreshProviders: refreshRouterProviders,
    addProvider,
    removeProvider,
  } = useIntelligentRouter();

  const { providers: allProviders, refreshProviders: refreshAllProviders } =
    useAIProvider();

  // Local state for form editing
  const [editConfig, setEditConfig] = useState<{
    load_balancing_strategy: string;
    fallback_strategy: string;
    cost_optimization_enabled: boolean;
    capability_matching_enabled: boolean;
    max_retries: number;
  } | null>(null);

  const [availableProviersToAdd, setAvailableProvidersToAdd] = useState<
    string[]
  >([]);

  useEffect(() => {
    refreshConfig();
    refreshStats();
    refreshRouterProviders();
    refreshAllProviders();
  }, [
    refreshConfig,
    refreshStats,
    refreshRouterProviders,
    refreshAllProviders,
  ]);

  useEffect(() => {
    if (config) {
      setEditConfig(config);
    }
  }, [config]);

  useEffect(() => {
    // Calculate available providers that are NOT in the router
    const routerProviderSet = new Set(routerProviders);
    const available = allProviders
      .filter((p) => !routerProviderSet.has(p.id))
      .map((p) => p.id);
    setAvailableProvidersToAdd(available);
  }, [allProviders, routerProviders]);

  const handleConfigChange = (key: string, value: any) => {
    setEditConfig((prev) => (prev ? { ...prev, [key]: value } : null));
  };

  const handleSaveConfig = async () => {
    if (!editConfig) return;
    try {
      await updateConfig(editConfig);
    } catch (err) {
      console.error("Failed to update config:", err);
    }
  };

  const handleAddProvider = async (providerId: string) => {
    try {
      await addProvider(providerId);
    } catch (err) {
      console.error("Failed to add provider:", err);
    }
  };

  const handleRemoveProvider = async (providerId: string) => {
    if (
      confirm(`Are you sure you want to remove ${providerId} from the router?`)
    ) {
      try {
        await removeProvider(providerId);
      } catch (err) {
        console.error("Failed to remove provider:", err);
      }
    }
  };

  if (isLoading && !config) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
      </div>
    );
  }

  return (
    <div className="backdrop-blur-md bg-white/80 rounded-xl shadow-lg p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold">Intelligent Router Configuration</h2>
        <button
          onClick={() => {
            refreshConfig();
            refreshStats();
            refreshRouterProviders();
            refreshAllProviders();
          }}
          className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600"
        >
          Refresh
        </button>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Configuration Form */}
        <div className="space-y-6">
          <h3 className="text-lg font-semibold border-b pb-2">
            Strategy Settings
          </h3>

          {editConfig && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Load Balancing Strategy
                </label>
                <select
                  value={editConfig.load_balancing_strategy}
                  onChange={(e) =>
                    handleConfigChange(
                      "load_balancing_strategy",
                      e.target.value,
                    )
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                >
                  <option value="round_robin">Round Robin</option>
                  <option value="random">Random</option>
                  <option value="lowest_latency">Lowest Latency</option>
                  <option value="lowest_cost">Lowest Cost</option>
                  <option value="highest_throughput">Highest Throughput</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Fallback Strategy
                </label>
                <select
                  value={editConfig.fallback_strategy}
                  onChange={(e) =>
                    handleConfigChange("fallback_strategy", e.target.value)
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                >
                  <option value="next_available">Next Available</option>
                  <option value="cheapest">Cheapest</option>
                  <option value="most_capable">Most Capable</option>
                  <option value="best_performance">Best Performance</option>
                </select>
              </div>

              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="cost_opt"
                  checked={editConfig.cost_optimization_enabled}
                  onChange={(e) =>
                    handleConfigChange(
                      "cost_optimization_enabled",
                      e.target.checked,
                    )
                  }
                  className="w-4 h-4 text-blue-600 rounded focus:ring-2 focus:ring-blue-500"
                />
                <label
                  htmlFor="cost_opt"
                  className="text-sm font-medium text-gray-700"
                >
                  Enable Cost Optimization
                </label>
              </div>

              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="cap_match"
                  checked={editConfig.capability_matching_enabled}
                  onChange={(e) =>
                    handleConfigChange(
                      "capability_matching_enabled",
                      e.target.checked,
                    )
                  }
                  className="w-4 h-4 text-blue-600 rounded focus:ring-2 focus:ring-blue-500"
                />
                <label
                  htmlFor="cap_match"
                  className="text-sm font-medium text-gray-700"
                >
                  Enable Capability Matching
                </label>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Max Retries
                </label>
                <input
                  type="number"
                  value={editConfig.max_retries}
                  onChange={(e) =>
                    handleConfigChange("max_retries", parseInt(e.target.value))
                  }
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                  min="0"
                  max="5"
                />
              </div>

              <button
                onClick={handleSaveConfig}
                className="w-full px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 font-medium"
              >
                Save Configuration
              </button>
            </div>
          )}
        </div>

        {/* Router Stats & Providers */}
        <div className="space-y-6">
          <h3 className="text-lg font-semibold border-b pb-2">Router Status</h3>

          {stats ? (
            <div className="grid grid-cols-3 gap-4 mb-6">
              <div className="p-3 bg-blue-50 border border-blue-200 rounded-lg text-center">
                <div className="text-2xl font-bold text-blue-700">
                  {stats.total_providers}
                </div>
                <div className="text-xs text-gray-600 uppercase">Providers</div>
              </div>
              <div className="p-3 bg-green-50 border border-green-200 rounded-lg text-center">
                <div className="text-2xl font-bold text-green-700">
                  {stats.healthy_providers}
                </div>
                <div className="text-xs text-gray-600 uppercase">Healthy</div>
              </div>
              <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-center">
                <div className="text-2xl font-bold text-red-700">
                  {stats.circuit_breakers_open}
                </div>
                <div className="text-xs text-gray-600 uppercase">Tripped</div>
              </div>
            </div>
          ) : (
            <p className="text-gray-500 italic">No stats available</p>
          )}

          <h3 className="text-lg font-semibold border-b pb-2 mt-6">
            Active Providers
          </h3>
          <div className="space-y-2 max-h-48 overflow-y-auto">
            {routerProviders.length === 0 ? (
              <p className="text-gray-500 italic p-4 text-center">
                No providers in router pool
              </p>
            ) : (
              routerProviders.map((pid) => (
                <div
                  key={pid}
                  className="flex justify-between items-center p-3 bg-gray-50 border border-gray-200 rounded-lg"
                >
                  <span className="font-medium">{pid}</span>
                  <button
                    onClick={() => handleRemoveProvider(pid)}
                    className="text-red-500 hover:text-red-700 text-sm font-medium"
                  >
                    Remove
                  </button>
                </div>
              ))
            )}
          </div>

          <h3 className="text-lg font-semibold border-b pb-2 mt-6">
            Add Provider
          </h3>
          <div className="flex gap-2">
            <select
              className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              onChange={(e) => {
                if (e.target.value) handleAddProvider(e.target.value);
              }}
              value=""
            >
              <option value="">Select provider to add...</option>
              {availableProviersToAdd.map((pid) => (
                <option key={pid} value={pid}>
                  {pid}
                </option>
              ))}
            </select>
          </div>
          {availableProviersToAdd.length === 0 && (
            <p className="text-xs text-gray-500 mt-1">
              All registered providers are already in the router pool.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

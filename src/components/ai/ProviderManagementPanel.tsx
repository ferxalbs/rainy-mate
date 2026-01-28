// Rainy Cowork - Provider Management Panel (PHASE 3)
// UI component for managing AI providers

import React, { useState, useEffect } from 'react';
import { useAIProvider } from '../../hooks/useAIProvider';
import type { ProviderInfo, RegisterProviderRequest } from '../../services/tauri';

export function ProviderManagementPanel() {
    const {
        providers,
        defaultProvider,
        isLoading,
        error,
        providerCount,
        refreshProviders,
        registerProvider,
        unregisterProvider,
        setDefaultProvider,
        testProviderConnection,
        getProviderCapabilities,
    } = useAIProvider();

    const [showRegisterForm, setShowRegisterForm] = useState(false);
    const [selectedProvider, setSelectedProvider] = useState<ProviderInfo | null>(null);
    const [registerForm, setRegisterForm] = useState<RegisterProviderRequest>({
        id: '',
        provider_type: 'rainy-sdk',
        api_key: '',
        base_url: '',
        model: 'gemini-pro',
        enabled: true,
        priority: 1,
        rate_limit: undefined,
        timeout: 30000,
    });
    const [connectionTestResults, setConnectionTestResults] = useState<Map<string, string>>(new Map());

    useEffect(() => {
        refreshProviders();
    }, [refreshProviders]);

    const handleRegister = async (e: React.FormEvent) => {
        e.preventDefault();
        try {
            await registerProvider(registerForm);
            setShowRegisterForm(false);
            setRegisterForm({
                id: '',
                provider_type: 'rainy-sdk',
                api_key: '',
                base_url: '',
                model: 'gemini-pro',
                enabled: true,
                priority: 1,
                rate_limit: undefined,
                timeout: 30000,
            });
        } catch (err) {
            console.error('Failed to register provider:', err);
        }
    };

    const handleUnregister = async (id: string) => {
        if (confirm('Are you sure you want to unregister this provider?')) {
            try {
                await unregisterProvider(id);
            } catch (err) {
                console.error('Failed to unregister provider:', err);
            }
        }
    };

    const handleSetDefault = async (id: string) => {
        try {
            await setDefaultProvider(id);
        } catch (err) {
            console.error('Failed to set default provider:', err);
        }
    };

    const handleTestConnection = async (id: string) => {
        try {
            const result = await testProviderConnection(id);
            setConnectionTestResults(prev => new Map(prev).set(id, result));
        } catch (err) {
            console.error('Connection test failed:', err);
            setConnectionTestResults(prev => new Map(prev).set(id, 'Failed'));
        }
    };

    const getHealthColor = (health: string) => {
        switch (health) {
            case 'Healthy':
                return 'text-green-500';
            case 'Degraded':
                return 'text-yellow-500';
            case 'Unhealthy':
                return 'text-red-500';
            default:
                return 'text-gray-500';
        }
    };

    return (
        <div className="backdrop-blur-md bg-white/80 rounded-xl shadow-lg p-6">
            <div className="flex items-center justify-between mb-6">
                <h2 className="text-2xl font-bold">AI Provider Management</h2>
                <div className="flex items-center gap-4">
                    <span className="text-sm text-gray-600">
                        {providerCount} provider{providerCount !== 1 ? 's' : ''} registered
                    </span>
                    <button
                        onClick={() => refreshProviders()}
                        disabled={isLoading}
                        className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50"
                    >
                        {isLoading ? 'Refreshing...' : 'Refresh'}
                    </button>
                    <button
                        onClick={() => setShowRegisterForm(true)}
                        className="px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600"
                    >
                        Add Provider
                    </button>
                </div>
            </div>

            {error && (
                <div className="mb-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded-lg">
                    {error}
                </div>
            )}

            {showRegisterForm && (
                <div className="mb-6 p-6 bg-gray-50 rounded-lg border border-gray-200">
                    <h3 className="text-lg font-semibold mb-4">Register New Provider</h3>
                    <form onSubmit={handleRegister} className="space-y-4">
                        <div>
                            <label className="block text-sm font-medium text-gray-700 mb-1">
                                Provider ID
                            </label>
                            <input
                                type="text"
                                value={registerForm.id}
                                onChange={(e) => setRegisterForm({ ...registerForm, id: e.target.value })}
                                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                required
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium text-gray-700 mb-1">
                                Provider Type
                            </label>
                            <select
                                value={registerForm.provider_type}
                                onChange={(e) => setRegisterForm({ ...registerForm, provider_type: e.target.value })}
                                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                            >
                                <option value="rainy-sdk">Rainy SDK</option>
                                <option value="openai">OpenAI</option>
                                <option value="anthropic">Anthropic</option>
                                <option value="google">Google</option>
                                <option value="xai">xAI</option>
                                <option value="local">Local</option>
                                <option value="custom">Custom</option>
                            </select>
                        </div>
                        <div>
                            <label className="block text-sm font-medium text-gray-700 mb-1">
                                API Key
                            </label>
                            <input
                                type="password"
                                value={registerForm.api_key}
                                onChange={(e) => setRegisterForm({ ...registerForm, api_key: e.target.value })}
                                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                            />
                        </div>
                        <div>
                            <label className="block text-sm font-medium text-gray-700 mb-1">
                                Model
                            </label>
                            <input
                                type="text"
                                value={registerForm.model}
                                onChange={(e) => setRegisterForm({ ...registerForm, model: e.target.value })}
                                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                required
                            />
                        </div>
                        <div className="flex gap-4">
                            <div className="flex-1">
                                <label className="block text-sm font-medium text-gray-700 mb-1">
                                    Priority
                                </label>
                                <input
                                    type="number"
                                    value={registerForm.priority}
                                    onChange={(e) => setRegisterForm({ ...registerForm, priority: parseInt(e.target.value) })}
                                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                    min="1"
                                    required
                                />
                            </div>
                            <div className="flex-1">
                                <label className="block text-sm font-medium text-gray-700 mb-1">
                                    Timeout (ms)
                                </label>
                                <input
                                    type="number"
                                    value={registerForm.timeout}
                                    onChange={(e) => setRegisterForm({ ...registerForm, timeout: parseInt(e.target.value) })}
                                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
                                    min="1000"
                                    required
                                />
                            </div>
                        </div>
                        <div className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                id="enabled"
                                checked={registerForm.enabled}
                                onChange={(e) => setRegisterForm({ ...registerForm, enabled: e.target.checked })}
                                className="w-4 h-4 text-blue-600 rounded focus:ring-2 focus:ring-blue-500"
                            />
                            <label htmlFor="enabled" className="text-sm font-medium text-gray-700">
                                Enabled
                            </label>
                        </div>
                        <div className="flex gap-2">
                            <button
                                type="submit"
                                className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600"
                            >
                                Register
                            </button>
                            <button
                                type="button"
                                onClick={() => setShowRegisterForm(false)}
                                className="px-4 py-2 bg-gray-300 text-gray-700 rounded-lg hover:bg-gray-400"
                            >
                                Cancel
                            </button>
                        </div>
                    </form>
                </div>
            )}

            <div className="space-y-4">
                {providers.length === 0 ? (
                    <div className="text-center py-12 text-gray-500">
                        <p className="text-lg mb-2">No providers registered</p>
                        <p className="text-sm">Add a provider to get started</p>
                    </div>
                ) : (
                    providers.map((provider) => (
                        <div
                            key={provider.id}
                            className={`p-4 rounded-lg border ${
                                provider.enabled
                                    ? 'bg-white border-gray-200'
                                    : 'bg-gray-50 border-gray-300'
                            }`}
                        >
                            <div className="flex items-start justify-between">
                                <div className="flex-1">
                                    <div className="flex items-center gap-2 mb-2">
                                        <h3 className="text-lg font-semibold">{provider.id}</h3>
                                        {defaultProvider?.id === provider.id && (
                                            <span className="px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded-full">
                                                Default
                                            </span>
                                        )}
                                        {!provider.enabled && (
                                            <span className="px-2 py-1 bg-gray-200 text-gray-700 text-xs rounded-full">
                                                Disabled
                                            </span>
                                        )}
                                    </div>
                                    <div className="grid grid-cols-2 gap-2 text-sm">
                                        <div>
                                            <span className="text-gray-600">Type:</span>{' '}
                                            <span className="font-medium">{provider.provider_type}</span>
                                        </div>
                                        <div>
                                            <span className="text-gray-600">Model:</span>{' '}
                                            <span className="font-medium">{provider.model}</span>
                                        </div>
                                        <div>
                                            <span className="text-gray-600">Priority:</span>{' '}
                                            <span className="font-medium">{provider.priority}</span>
                                        </div>
                                        <div>
                                            <span className="text-gray-600">Health:</span>{' '}
                                            <span className={`font-medium ${getHealthColor(provider.health)}`}>
                                                {provider.health}
                                            </span>
                                        </div>
                                    </div>
                                    <div className="mt-2 flex flex-wrap gap-2">
                                        {provider.capabilities.chat && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Chat
                                            </span>
                                        )}
                                        {provider.capabilities.embeddings && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Embeddings
                                            </span>
                                        )}
                                        {provider.capabilities.streaming && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Streaming
                                            </span>
                                        )}
                                        {provider.capabilities.web_search && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Web Search
                                            </span>
                                        )}
                                        {provider.capabilities.image_generation && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Image Gen
                                            </span>
                                        )}
                                        {provider.capabilities.function_calling && (
                                            <span className="px-2 py-1 bg-green-100 text-green-800 text-xs rounded">
                                                Functions
                                            </span>
                                        )}
                                    </div>
                                </div>
                                <div className="flex flex-col gap-2 ml-4">
                                    <button
                                        onClick={() => handleTestConnection(provider.id)}
                                        className="px-3 py-1 bg-yellow-500 text-white text-sm rounded hover:bg-yellow-600"
                                    >
                                        Test
                                    </button>
                                    {defaultProvider?.id !== provider.id && (
                                        <button
                                            onClick={() => handleSetDefault(provider.id)}
                                            className="px-3 py-1 bg-blue-500 text-white text-sm rounded hover:bg-blue-600"
                                        >
                                            Set Default
                                        </button>
                                    )}
                                    <button
                                        onClick={() => setSelectedProvider(provider)}
                                        className="px-3 py-1 bg-gray-500 text-white text-sm rounded hover:bg-gray-600"
                                    >
                                        Details
                                    </button>
                                    <button
                                        onClick={() => handleUnregister(provider.id)}
                                        className="px-3 py-1 bg-red-500 text-white text-sm rounded hover:bg-red-600"
                                    >
                                        Remove
                                    </button>
                                </div>
                            </div>
                            {connectionTestResults.get(provider.id) && (
                                <div className="mt-2 p-2 bg-blue-50 border border-blue-200 rounded text-sm">
                                    <span className="font-medium">Connection Test:</span>{' '}
                                    {connectionTestResults.get(provider.id)}
                                </div>
                            )}
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

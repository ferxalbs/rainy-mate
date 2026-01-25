/**
 * useCoworkKeys Hook
 *
 * React hook for managing Cowork API keys.
 * Provides CRUD operations for ra-cowork{48hex} keys.
 */

import { useCallback, useEffect, useState } from 'react';

// API base URL - in Tauri we call through the backend
const API_BASE_URL = import.meta.env.VITE_API_URL || 'https://api.enosislabs.com';

export interface CoworkApiKey {
    id: string;
    name: string;
    isActive: boolean;
    lastUsed: string | null;
    createdAt: string;
}

interface UseCoworkKeysResult {
    keys: CoworkApiKey[];
    isLoading: boolean;
    error: string | null;

    // Actions
    createKey: (name: string) => Promise<{ key: string; id: string } | null>;
    revokeKey: (id: string) => Promise<boolean>;
    refresh: () => Promise<void>;
}

export function useCoworkKeys(): UseCoworkKeysResult {
    const [keys, setKeys] = useState<CoworkApiKey[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    // Get JWT token from localStorage (set by auth flow)
    const getAuthToken = useCallback(() => {
        return localStorage.getItem('accessToken');
    }, []);

    const refresh = useCallback(async () => {
        const token = getAuthToken();
        if (!token) {
            setError('Not authenticated');
            setIsLoading(false);
            return;
        }

        setIsLoading(true);
        setError(null);

        try {
            const response = await fetch(`${API_BASE_URL}/api/v1/cowork/keys`, {
                method: 'GET',
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                },
            });

            if (!response.ok) {
                throw new Error('Failed to fetch keys');
            }

            const data = await response.json();
            setKeys(data.keys || []);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load keys');
        } finally {
            setIsLoading(false);
        }
    }, [getAuthToken]);

    useEffect(() => {
        refresh();
    }, [refresh]);

    const createKey = useCallback(async (name: string): Promise<{ key: string; id: string } | null> => {
        const token = getAuthToken();
        if (!token) {
            setError('Not authenticated');
            return null;
        }

        try {
            const response = await fetch(`${API_BASE_URL}/api/v1/cowork/keys`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ name }),
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.error?.message || 'Failed to create key');
            }

            const data = await response.json();

            // Refresh the keys list
            await refresh();

            return {
                key: data.key,
                id: data.details.id,
            };
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to create key');
            return null;
        }
    }, [getAuthToken, refresh]);

    const revokeKey = useCallback(async (id: string): Promise<boolean> => {
        const token = getAuthToken();
        if (!token) {
            setError('Not authenticated');
            return false;
        }

        try {
            const response = await fetch(`${API_BASE_URL}/api/v1/cowork/keys/${id}`, {
                method: 'DELETE',
                headers: {
                    'Authorization': `Bearer ${token}`,
                },
            });

            if (!response.ok) {
                throw new Error('Failed to revoke key');
            }

            // Refresh the keys list
            await refresh();
            return true;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to revoke key');
            return false;
        }
    }, [getAuthToken, refresh]);

    return {
        keys,
        isLoading,
        error,
        createKey,
        revokeKey,
        refresh,
    };
}

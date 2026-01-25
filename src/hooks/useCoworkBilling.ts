/**
 * useCoworkBilling Hook
 *
 * React hook for Cowork subscription management.
 * Handles plan listing, checkout, and subscription status.
 */

import { useCallback, useEffect, useState } from 'react';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'https://api.enosislabs.com';

export interface CoworkPlan {
    id: string;
    name: string;
    price: number;
    usageLimit: number;
    modelAccessLevel: string;
    features: {
        web_research: boolean;
        document_export: boolean;
        image_analysis: boolean;
        priority_support: boolean;
    } | null;
    hasStripePrice: boolean;
}

export interface CoworkSubscription {
    hasSubscription: boolean;
    plan: string;
    planName?: string;
    status?: string;
    currentPeriodEnd?: string;
    usageThisMonth?: number;
    creditsUsedThisMonth?: number;
}

interface UseCoworkBillingResult {
    plans: CoworkPlan[];
    subscription: CoworkSubscription | null;
    isLoading: boolean;
    error: string | null;

    // Actions
    checkout: (planId: string) => Promise<string | null>;
    openPortal: () => Promise<string | null>;
    refresh: () => Promise<void>;
}

export function useCoworkBilling(): UseCoworkBillingResult {
    const [plans, setPlans] = useState<CoworkPlan[]>([]);
    const [subscription, setSubscription] = useState<CoworkSubscription | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const getAuthToken = useCallback(() => {
        return localStorage.getItem('accessToken');
    }, []);

    const refresh = useCallback(async () => {
        setIsLoading(true);
        setError(null);

        try {
            // Fetch plans (public endpoint)
            const plansResponse = await fetch(`${API_BASE_URL}/api/v1/cowork/billing/plans`);
            if (plansResponse.ok) {
                const plansData = await plansResponse.json();
                setPlans(plansData.plans || []);
            }

            // Fetch subscription status (requires auth)
            const token = getAuthToken();
            if (token) {
                const subResponse = await fetch(`${API_BASE_URL}/api/v1/cowork/billing/subscription`, {
                    headers: { 'Authorization': `Bearer ${token}` },
                });
                if (subResponse.ok) {
                    const subData = await subResponse.json();
                    setSubscription(subData);
                }
            }
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load billing info');
        } finally {
            setIsLoading(false);
        }
    }, [getAuthToken]);

    useEffect(() => {
        refresh();
    }, [refresh]);

    const checkout = useCallback(async (planId: string): Promise<string | null> => {
        const token = getAuthToken();
        if (!token) {
            setError('Not authenticated');
            return null;
        }

        try {
            const response = await fetch(`${API_BASE_URL}/api/v1/cowork/billing/checkout-session`, {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${token}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ planId }),
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.error?.message || 'Failed to create checkout');
            }

            const data = await response.json();
            return data.url;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to start checkout');
            return null;
        }
    }, [getAuthToken]);

    const openPortal = useCallback(async (): Promise<string | null> => {
        const token = getAuthToken();
        if (!token) {
            setError('Not authenticated');
            return null;
        }

        try {
            const response = await fetch(`${API_BASE_URL}/api/v1/cowork/billing/portal`, {
                method: 'POST',
                headers: { 'Authorization': `Bearer ${token}` },
            });

            if (!response.ok) {
                throw new Error('Failed to open billing portal');
            }

            const data = await response.json();
            return data.url;
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to open portal');
            return null;
        }
    }, [getAuthToken]);

    return {
        plans,
        subscription,
        isLoading,
        error,
        checkout,
        openPortal,
        refresh,
    };
}

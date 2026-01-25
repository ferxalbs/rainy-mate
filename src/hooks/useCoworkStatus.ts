// Rainy Cowork - useCoworkStatus Hook
// React hook for Cowork subscription status and usage tracking

import { useCallback, useEffect, useState } from 'react';
import * as tauri from '../services/tauri';
import type { CoworkStatus, CoworkPlan } from '../services/tauri';

interface UseCoworkStatusResult {
    status: CoworkStatus | null;
    isLoading: boolean;
    error: string | null;

    // Computed properties
    hasPaidPlan: boolean;
    plan: CoworkPlan;
    planName: string;
    isValid: boolean;

    // Usage helpers
    usagePercent: number;
    remainingUses: number;
    isOverLimit: boolean;
    isOverBudget: boolean;

    // Feature helpers
    canUseWebResearch: boolean;
    canUseDocumentExport: boolean;
    canUseImageAnalysis: boolean;
    hasPrioritySupport: boolean;

    // Actions
    refresh: () => Promise<void>;
    canUseFeature: (feature: string) => Promise<boolean>;
}

export function useCoworkStatus(): UseCoworkStatusResult {
    const [status, setStatus] = useState<CoworkStatus | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const refresh = useCallback(async () => {
        setIsLoading(true);
        setError(null);
        try {
            const coworkStatus = await tauri.getCoworkStatus();
            setStatus(coworkStatus);
        } catch (err) {
            setError(err instanceof Error ? err.message : String(err));
            // Set default free status on error
            setStatus({
                has_paid_plan: false,
                plan: 'free',
                plan_name: 'Free',
                is_valid: true,
                models: [],
                features: {
                    web_research: false,
                    document_export: false,
                    image_analysis: false,
                    priority_support: false,
                },
                usage: {
                    used: 0,
                    limit: 30,
                    credits_used: 0,
                    credits_ceiling: 0,
                    resets_at: '',
                },
                upgrade_message: null,
            });
        } finally {
            setIsLoading(false);
        }
    }, []);

    useEffect(() => {
        refresh();
    }, [refresh]);

    const canUseFeature = useCallback(async (feature: string): Promise<boolean> => {
        try {
            return await tauri.canUseFeature(feature);
        } catch {
            return false;
        }
    }, []);

    // Computed values
    const hasPaidPlan = status?.has_paid_plan ?? false;
    const plan = (status?.plan ?? 'free') as CoworkPlan;
    const planName = status?.plan_name ?? 'Free';
    const isValid = status?.is_valid ?? true;

    const usage = status?.usage ?? { used: 0, limit: 30, credits_used: 0, credits_ceiling: 0, resets_at: '' };
    const usagePercent = usage.limit > 0 ? Math.round((usage.used / usage.limit) * 100) : 0;
    const remainingUses = Math.max(0, usage.limit - usage.used);
    const isOverLimit = usage.used >= usage.limit;
    const isOverBudget = usage.credits_ceiling > 0 && usage.credits_used >= usage.credits_ceiling;

    const features = status?.features ?? {
        web_research: false,
        document_export: false,
        image_analysis: false,
        priority_support: false,
    };

    return {
        status,
        isLoading,
        error,

        hasPaidPlan,
        plan,
        planName,
        isValid,

        usagePercent,
        remainingUses,
        isOverLimit,
        isOverBudget,

        canUseWebResearch: features.web_research,
        canUseDocumentExport: features.document_export,
        canUseImageAnalysis: features.image_analysis,
        hasPrioritySupport: features.priority_support,

        refresh,
        canUseFeature,
    };
}

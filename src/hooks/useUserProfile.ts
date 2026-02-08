import { useCallback, useEffect, useState } from "react";
import { getUserProfile, setUserProfile, type UserProfile } from "../services/tauri";

const FALLBACK_PROFILE: UserProfile = {
  displayName: "Rainy User",
  email: "",
  organization: "",
  role: "Builder",
};

export function useUserProfile() {
  const [profile, setProfile] = useState<UserProfile>(FALLBACK_PROFILE);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const current = await getUserProfile();
      setProfile(current);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const saveProfile = useCallback(async (nextProfile: UserProfile) => {
    await setUserProfile(nextProfile);
    setProfile(nextProfile);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return {
    profile,
    isLoading,
    error,
    refresh,
    saveProfile,
  };
}

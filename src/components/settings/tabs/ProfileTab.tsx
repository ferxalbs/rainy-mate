import { useState } from "react";
import { User, Mail, Building2, Briefcase, Check, Loader2 } from "lucide-react";
import { useUserProfile } from "../../../hooks/useUserProfile";
import type { UserProfile } from "../../../services/tauri";
import { Input, Button, Card } from "@heroui/react";

export function ProfileTab() {
  const {
    profile,
    isLoading: isLoadingProfile,
    saveProfile,
  } = useUserProfile();
  const [isSavingProfile, setIsSavingProfile] = useState(false);
  const [editedProfile, setEditedProfile] = useState<Partial<UserProfile>>({});

  const profileForm = { ...profile, ...editedProfile };

  const updateProfileField = (key: keyof UserProfile, value: string) => {
    setEditedProfile((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const handleSaveProfile = async () => {
    setIsSavingProfile(true);
    try {
      await saveProfile(profileForm as UserProfile);
    } finally {
      setIsSavingProfile(false);
    }
  };

  if (isLoadingProfile) {
    return (
      <div className="flex flex-col items-center justify-center p-20 space-y-4 animate-in fade-in duration-500">
        <Loader2 className="size-8 text-primary animate-spin opacity-50" />
        <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground/40">
          Syncing Identity...
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-10 animate-in fade-in duration-500">
      <Card className="p-8 bg-muted/10 border-border/5 backdrop-blur-xl rounded-3xl space-y-8 shadow-2xl shadow-primary/5">
        <div className="flex items-center justify-between pb-6 border-b border-border/10">
          <div className="space-y-1.5">
            <h3 className="text-sm font-bold tracking-tight text-foreground uppercase opacity-70 flex items-center gap-2">
              <User className="size-4 text-primary" />
              User Profile
            </h3>
            <p className="text-xs text-muted-foreground max-w-md">
              Your identity details are encrypted and stored locally.
            </p>
          </div>
          <div className="hidden sm:flex size-12 rounded-2xl bg-primary/10 items-center justify-center border border-primary/20 shadow-inner">
            <User className="size-6 text-primary" />
          </div>
        </div>

        <div className="grid gap-8">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-8">
            <div className="space-y-3 group/field">
              <label className="block text-xs font-bold uppercase tracking-widest text-muted-foreground/60 transition-colors group-focus-within/field:text-primary">
                Display Name
              </label>
              <div className="relative">
                <User className="absolute left-4 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/40 z-10" />
                <Input
                  className="h-12 pl-11 rounded-2xl bg-background/50 border border-border/10 focus-within:ring-1 focus-within:ring-primary/20 focus-within:border-primary/50 transition-all shadow-none w-full"
                  placeholder="Creative Name"
                  value={profileForm.displayName || ""}
                  onChange={(e) => updateProfileField("displayName", e.target.value)}
                />
              </div>
            </div>

            <div className="space-y-3 group/field">
              <label className="block text-xs font-bold uppercase tracking-widest text-muted-foreground/60 transition-colors group-focus-within/field:text-primary">
                Email Address
              </label>
              <div className="relative">
                <Mail className="absolute left-4 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/40 z-10" />
                <Input
                  className="h-12 pl-11 rounded-2xl bg-background/50 border border-border/10 focus-within:ring-1 focus-within:ring-primary/20 focus-within:border-primary/50 transition-all shadow-none w-full"
                  placeholder="name@domain.com"
                  type="email"
                  value={profileForm.email || ""}
                  onChange={(e) => updateProfileField("email", e.target.value)}
                />
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-8">
            <div className="space-y-3 group/field">
              <label className="block text-xs font-bold uppercase tracking-widest text-muted-foreground/60 transition-colors group-focus-within/field:text-primary">
                Organization
              </label>
              <div className="relative">
                <Building2 className="absolute left-4 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/40 z-10" />
                <Input
                  className="h-12 pl-11 rounded-2xl bg-background/50 border border-border/10 focus-within:ring-1 focus-within:ring-primary/20 focus-within:border-primary/50 transition-all shadow-none w-full"
                  placeholder="Studio or Company"
                  value={profileForm.organization || ""}
                  onChange={(e) => updateProfileField("organization", e.target.value)}
                />
              </div>
            </div>

            <div className="space-y-3 group/field">
              <label className="block text-xs font-bold uppercase tracking-widest text-muted-foreground/60 transition-colors group-focus-within/field:text-primary">
                Role / Title
              </label>
              <div className="relative">
                <Briefcase className="absolute left-4 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/40 z-10" />
                <Input
                  className="h-12 pl-11 rounded-2xl bg-background/50 border border-border/10 focus-within:ring-1 focus-within:ring-primary/20 focus-within:border-primary/50 transition-all shadow-none w-full"
                  placeholder="e.g. Lead Designer"
                  value={profileForm.role || ""}
                  onChange={(e) => updateProfileField("role", e.target.value)}
                />
              </div>
            </div>
          </div>
        </div>

        <div className="flex flex-col sm:flex-row items-center justify-between gap-6 pt-8 border-t border-border/10">
          <div className="flex items-center gap-3">
            <div className="size-2 rounded-full bg-emerald-500 animate-pulse" />
            <p className="text-[10px] font-medium uppercase tracking-widest text-muted-foreground/50">
              Identity Verified & Locally Encrypted
            </p>
          </div>
          <Button
            onPress={handleSaveProfile}
            isDisabled={isSavingProfile}
            variant="primary"
            className="w-full sm:w-auto h-12 px-8 rounded-2xl font-bold shadow-xl shadow-primary/20 hover:scale-[1.02] active:scale-95 transition-all flex items-center gap-2"
          >
            {isSavingProfile ? (
              <>
                <Loader2 className="size-4 animate-spin" />
                Applying Changes...
              </>
            ) : (
              <>
                Commit Profile
                <Check className="size-4" />
              </>
            )}
          </Button>
        </div>
      </Card>
    </div>
  );
}

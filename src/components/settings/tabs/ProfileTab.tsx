import { useState, useEffect } from "react";
import { Spinner, Label, TextField, Input, Button } from "@heroui/react";
import { User, Mail, Building2, Briefcase, Check } from "lucide-react";
import { useUserProfile } from "../../../hooks/useUserProfile";
import type { UserProfile } from "../../../services/tauri";

export function ProfileTab() {
  const {
    profile,
    isLoading: isLoadingProfile,
    saveProfile,
  } = useUserProfile();
  const [isSavingProfile, setIsSavingProfile] = useState(false);
  const [profileForm, setProfileForm] = useState<UserProfile>({
    displayName: "",
    email: "",
    organization: "",
    role: "",
  });

  useEffect(() => {
    setProfileForm(profile);
  }, [profile]);

  const updateProfileField = (key: keyof UserProfile, value: string) => {
    setProfileForm((prev) => ({
      ...prev,
      [key]: value,
    }));
  };

  const handleSaveProfile = async () => {
    setIsSavingProfile(true);
    try {
      await saveProfile(profileForm);
    } finally {
      setIsSavingProfile(false);
    }
  };

  if (isLoadingProfile) {
    return (
      <div className="flex items-center justify-center p-12 text-muted-foreground animate-pulse">
        <Spinner size="lg" color="current" />
      </div>
    );
  }

  return (
    <div className="p-6 rounded-2xl border bg-card/40 backdrop-blur-md border-border/40 shadow-sm space-y-6 animate-appear">
      {/* Header */}
      <div className="flex items-start justify-between border-b border-border/30 pb-4">
        <div className="space-y-1">
          <h3 className="text-base font-semibold text-foreground flex items-center gap-2">
            <User className="size-4 text-primary" />
            User Identity
          </h3>
          <p className="text-sm text-muted-foreground">
            This profile is used by the desktop app and cloud bridge for
            personalization.
          </p>
        </div>
        <div className="hidden sm:block">
          <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
            <User className="size-4 text-primary" />
          </div>
        </div>
      </div>

      {/* Form Grid */}
      <div className="grid gap-5">
        <div className="space-y-2 group">
          <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
            Display Name
          </Label>
          <TextField
            aria-label="Display Name"
            name="profile-display-name"
            className="w-full"
            onChange={(value) => updateProfileField("displayName", value)}
          >
            <div className="relative">
              <User className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
              <Input
                className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                placeholder="Create a display name..."
                value={profileForm.displayName}
              />
            </div>
          </TextField>
        </div>

        <div className="space-y-2 group">
          <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
            Email Address
          </Label>
          <TextField
            aria-label="Email Address"
            name="profile-email"
            className="w-full"
            onChange={(value) => updateProfileField("email", value)}
          >
            <div className="relative">
              <Mail className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
              <Input
                className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                placeholder="name@company.com"
                type="email"
                value={profileForm.email}
              />
            </div>
          </TextField>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
          <div className="space-y-2 group">
            <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
              Organization
            </Label>
            <TextField
              aria-label="Organization"
              name="profile-organization"
              className="w-full"
              onChange={(value) => updateProfileField("organization", value)}
            >
              <div className="relative">
                <Building2 className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                <Input
                  className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                  placeholder="Company Name"
                  value={profileForm.organization}
                />
              </div>
            </TextField>
          </div>

          <div className="space-y-2 group">
            <Label className="text-xs font-medium text-muted-foreground ml-1 group-focus-within:text-primary transition-colors">
              Role
            </Label>
            <TextField
              aria-label="Role"
              name="profile-role"
              className="w-full"
              onChange={(value) => updateProfileField("role", value)}
            >
              <div className="relative">
                <Briefcase className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground/50 z-10 pointer-events-none" />
                <Input
                  className="w-full h-11 rounded-xl border border-border/40 bg-muted/20 pl-10 pr-4 text-sm outline-none transition-all placeholder:text-muted-foreground/30 focus:border-primary/50 focus:bg-background focus:ring-4 focus:ring-primary/10 hover:bg-muted/30"
                  placeholder="e.g. Developer"
                  value={profileForm.role}
                />
              </div>
            </TextField>
          </div>
        </div>
      </div>

      {/* Actions Footer */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-4 pt-6 border-t border-border/30">
        <p className="text-xs text-muted-foreground flex items-center gap-1.5 opacity-70">
          <div className="size-1.5 rounded-full bg-emerald-500/50" />
          Changes are encrypted locally
        </p>
        <Button
          variant="primary"
          className="w-full sm:w-auto min-w-[140px] rounded-xl font-medium shadow-lg shadow-primary/20 hover:shadow-primary/30 active:scale-95 transition-all h-10"
          onPress={handleSaveProfile}
          isDisabled={isSavingProfile}
        >
          {isSavingProfile ? (
            <>
              <Spinner size="sm" color="current" className="mr-2" />
              Saving...
            </>
          ) : (
            <>
              Save Changes
              <Check className="size-4 ml-2" />
            </>
          )}
        </Button>
      </div>
    </div>
  );
}

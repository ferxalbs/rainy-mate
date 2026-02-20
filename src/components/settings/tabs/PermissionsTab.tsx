import { Label, Switch, Separator } from "@heroui/react";

export function PermissionsTab() {
  return (
    <div className="space-y-4">
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <Label className="font-medium">Notifications</Label>
            <p className="text-sm text-muted-foreground">
              Show task completion alerts
            </p>
          </div>
          <Switch defaultSelected>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>

        <Separator />

        <div className="flex items-center justify-between">
          <div>
            <Label className="font-medium">Auto-execute Tasks</Label>
            <p className="text-sm text-muted-foreground">
              Start tasks immediately
            </p>
          </div>
          <Switch>
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
          </Switch>
        </div>
      </div>
    </div>
  );
}

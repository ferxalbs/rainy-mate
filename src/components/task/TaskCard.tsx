import { Card, Button } from "@heroui/react";
import { Pause, Square, Eye, Loader2, CheckCircle2, XCircle, Timer, Play } from "lucide-react";
import type { Task } from "../../types";

interface TaskCardProps {
    task: Task;
    onPause?: (taskId: string) => void;
    onStop?: (taskId: string) => void;
    onViewDetails?: (taskId: string) => void;
}

const statusIcons = {
    queued: <Timer className="size-5 text-orange-500 shrink-0" />,
    running: <Loader2 className="size-5 text-blue-500 animate-spin shrink-0" />,
    paused: <Pause className="size-5 text-yellow-500 shrink-0" />,
    completed: <CheckCircle2 className="size-5 text-green-500 shrink-0" />,
    failed: <XCircle className="size-5 text-red-500 shrink-0" />,
    cancelled: <Square className="size-5 text-muted-foreground shrink-0" />,
};

const statusColors = {
    queued: "text-orange-500",
    running: "text-blue-500",
    paused: "text-yellow-500",
    completed: "text-green-500",
    failed: "text-red-500",
    cancelled: "text-muted-foreground",
};

const progressColors = {
    queued: "bg-orange-500",
    running: "bg-blue-500",
    paused: "bg-yellow-500",
    completed: "bg-green-500",
    failed: "bg-red-500",
    cancelled: "bg-muted-foreground",
};

export function TaskCard({ task, onPause, onStop, onViewDetails }: TaskCardProps) {
    const isActive = task.status === "running" || task.status === "paused";

    return (
        <Card className="w-full animate-task-appear" variant="secondary">
            <Card.Header>
                <div className="flex items-start justify-between w-full gap-3">
                    <div className="flex items-center gap-3 min-w-0">
                        {statusIcons[task.status]}
                        <div className="flex flex-col min-w-0">
                            <Card.Title className="text-sm sm:text-base truncate">{task.title}</Card.Title>
                            <Card.Description className="text-xs">
                                {task.provider.toUpperCase()} • {task.model}
                            </Card.Description>
                        </div>
                    </div>
                    <span className={`text-xs font-medium capitalize whitespace-nowrap ${statusColors[task.status]}`}>
                        {task.status}
                    </span>
                </div>
            </Card.Header>

            <Card.Content className="space-y-3">
                {/* Progress Bar */}
                <div className="space-y-1.5">
                    <div className="flex justify-between items-center text-xs">
                        <span className="text-muted-foreground">Progress</span>
                        <span className="font-medium">{task.progress}%</span>
                    </div>
                    <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
                        <div
                            className={`h-full transition-all duration-300 ease-out rounded-full ${progressColors[task.status]}`}
                            style={{ width: `${task.progress}%` }}
                        />
                    </div>
                </div>

                {/* Task Steps */}
                {task.steps && task.steps.length > 0 && (
                    <div className="text-xs text-muted-foreground">
                        Step {task.steps.filter((s) => s.status === "completed").length + 1} of{" "}
                        {task.steps.length}:{" "}
                        <span className="text-foreground font-medium">
                            {task.steps.find((s) => s.status === "running")?.name || "Waiting..."}
                        </span>
                    </div>
                )}

                {/* Error message */}
                {task.error && (
                    <p className="text-xs text-red-500 bg-red-500/10 p-2 rounded-lg">{task.error}</p>
                )}
            </Card.Content>

            {isActive && (
                <Card.Footer className="flex flex-wrap gap-2">
                    <Button
                        size="sm"
                        variant="secondary"
                        onPress={() => onPause?.(task.id)}
                    >
                        {task.status === "paused" ? <Play className="size-3.5" /> : <Pause className="size-3.5" />}
                        <span className="hidden sm:inline ml-1">{task.status === "paused" ? "Resume" : "Pause"}</span>
                    </Button>
                    <Button
                        size="sm"
                        variant="danger"
                        onPress={() => onStop?.(task.id)}
                    >
                        <Square className="size-3.5" />
                        <span className="hidden sm:inline ml-1">Stop</span>
                    </Button>
                    <Button
                        size="sm"
                        variant="ghost"
                        onPress={() => onViewDetails?.(task.id)}
                    >
                        <Eye className="size-3.5" />
                        <span className="hidden sm:inline ml-1">View</span>
                    </Button>
                </Card.Footer>
            )}
        </Card>
    );
}

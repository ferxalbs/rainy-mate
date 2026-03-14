// src/components/agents/AgentProgress.tsx
// Agent Progress Component
// Displays AI agent task progress with HeroUI v3
// Part of Phase 3 - Rainy MaTe

import { Card, Button, Spinner, Chip } from '@heroui/react';
import { CheckCircle, XCircle, Clock, Zap } from 'lucide-react';
import type { AgentTask, AgentTaskStatus } from '../../types/agent';

interface AgentProgressProps {
    task: AgentTask | null;
    onCancel?: () => void;
    onClear?: () => void;
}

/**
 * Status icon component
 */
function StatusIcon({ status }: { status: AgentTaskStatus }) {
    switch (status) {
        case 'running':
            return <Spinner size="sm" />;
        case 'completed':
            return <CheckCircle className="w-5 h-5 text-success" />;
        case 'failed':
            return <XCircle className="w-5 h-5 text-danger" />;
        case 'pending':
        default:
            return <Clock className="w-5 h-5 text-foreground-500" />;
    }
}

/**
 * Status chip color mapping (HeroUI v3 colors)
 */
function getStatusColor(status: AgentTaskStatus): 'accent' | 'success' | 'danger' | 'default' {
    switch (status) {
        case 'running':
            return 'accent';
        case 'completed':
            return 'success';
        case 'failed':
            return 'danger';
        default:
            return 'default';
    }
}

/**
 * CSS-based progress bar (HeroUI v3 doesn't include Progress component)
 */
function ProgressBar({ value, label }: { value: number; label?: string }) {
    return (
        <div className="w-full">
            {label && (
                <div className="flex justify-between mb-1 text-sm">
                    <span className="text-foreground-500">{label}</span>
                    <span className="text-foreground-500">{value}%</span>
                </div>
            )}
            <div className="w-full bg-surface-tertiary rounded-full h-2">
                <div
                    className="bg-accent h-2 rounded-full transition-all duration-300"
                    style={{ width: `${Math.min(100, Math.max(0, value))}%` }}
                />
            </div>
        </div>
    );
}

/**
 * Agent Progress Component
 * Shows real-time progress of AI agent tasks
 */
export function AgentProgress({ task, onCancel, onClear }: AgentProgressProps) {
    if (!task) return null;

    const isActive = task.status === 'running' || task.status === 'pending';
    const isComplete = task.status === 'completed' || task.status === 'failed';

    return (
        <Card className="w-full">
            <Card.Header className="flex justify-between items-center">
                <div className="flex items-center gap-3">
                    <Zap className="w-5 h-5 text-accent" />
                    <div>
                        <Card.Title>
                            {task.type === 'document' ? 'Document Generation' : 'Web Research'}
                        </Card.Title>
                        <Card.Description className="truncate max-w-md">
                            {task.prompt}
                        </Card.Description>
                    </div>
                </div>
                <Chip color={getStatusColor(task.status)} size="sm">
                    <StatusIcon status={task.status} />
                    <span className="ml-1 capitalize">{task.status}</span>
                </Chip>
            </Card.Header>

            <Card.Content className="space-y-4">
                {/* Progress bar */}
                {isActive && (
                    <ProgressBar
                        value={task.progress}
                        label={task.currentStep || 'Processing...'}
                    />
                )}

                {/* Result display */}
                {task.result && (
                    <div className="mt-4">
                        {task.result.success ? (
                            <div className="p-4 bg-success/10 rounded-lg">
                                <h4 className="font-medium text-success mb-2">
                                    Task Completed
                                </h4>
                                {task.result.content && (
                                    <pre className="text-sm whitespace-pre-wrap text-foreground-600 max-h-64 overflow-auto">
                                        {task.result.content.slice(0, 500)}
                                        {task.result.content.length > 500 && '...'}
                                    </pre>
                                )}
                            </div>
                        ) : (
                            <div className="p-4 bg-danger/10 rounded-lg">
                                <h4 className="font-medium text-danger mb-2">
                                    Task Failed
                                </h4>
                                <p className="text-sm text-danger/80">
                                    {task.result.error || 'An unknown error occurred'}
                                </p>
                            </div>
                        )}
                    </div>
                )}

                {/* Metadata */}
                <div className="flex gap-4 text-sm text-foreground-500">
                    {task.templateId && (
                        <span>Template: {task.templateId}</span>
                    )}
                    {task.result?.network && (
                        <span>Network: {task.result.network}</span>
                    )}
                    {task.completedAt && (
                        <span>
                            Duration: {Math.round((new Date(task.completedAt).getTime() - new Date(task.createdAt).getTime()) / 1000)}s
                        </span>
                    )}
                </div>
            </Card.Content>

            <Card.Footer className="flex justify-end gap-2">
                {isActive && onCancel && (
                    <Button variant="danger" size="sm" onPress={onCancel}>
                        Cancel
                    </Button>
                )}
                {isComplete && onClear && (
                    <Button variant="tertiary" size="sm" onPress={onClear}>
                        Clear
                    </Button>
                )}
            </Card.Footer>
        </Card>
    );
}

export default AgentProgress;

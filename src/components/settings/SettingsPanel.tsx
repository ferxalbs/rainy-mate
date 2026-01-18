// Rainy Cowork - Settings Panel
// AI Provider configuration with API key management

import { useState } from 'react';
import {
    Modal,
    Button,
    TextField,
    Input,
    Tabs,
    Switch,
    Label,
    Card,
    Separator,
} from '@heroui/react';
import { Settings, Key, Zap, Sparkles, Check, X, Eye, EyeOff } from 'lucide-react';
import { useAIProvider } from '../../hooks';
import { AI_PROVIDERS, type ProviderType } from '../../types';

interface SettingsPanelProps {
    isOpen: boolean;
    onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
    const {
        hasApiKey,
        validateApiKey,
        storeApiKey,
        deleteApiKey,
    } = useAIProvider();

    const [activeTab, setActiveTab] = useState('providers');
    const [apiKeyInputs, setApiKeyInputs] = useState<Record<string, string>>({});
    const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
    const [validationStatus, setValidationStatus] = useState<Record<string, 'idle' | 'validating' | 'valid' | 'invalid'>>({});
    const [savingStatus, setSavingStatus] = useState<Record<string, boolean>>({});

    // Provider ID mapping
    const getProviderId = (type: ProviderType) => type === 'rainyApi' ? 'rainy_api' : 'gemini';

    const handleApiKeyChange = (provider: ProviderType, value: string) => {
        setApiKeyInputs(prev => ({ ...prev, [provider]: value }));
        setValidationStatus(prev => ({ ...prev, [provider]: 'idle' }));
    };

    const handleValidateKey = async (provider: ProviderType) => {
        const key = apiKeyInputs[provider];
        if (!key?.trim()) return;

        setValidationStatus(prev => ({ ...prev, [provider]: 'validating' }));

        try {
            const providerId = getProviderId(provider);
            const isValid = await validateApiKey(providerId, key);
            setValidationStatus(prev => ({ ...prev, [provider]: isValid ? 'valid' : 'invalid' }));
        } catch {
            setValidationStatus(prev => ({ ...prev, [provider]: 'invalid' }));
        }
    };

    const handleSaveKey = async (provider: ProviderType) => {
        const key = apiKeyInputs[provider];
        if (!key?.trim()) return;

        setSavingStatus(prev => ({ ...prev, [provider]: true }));

        try {
            const providerId = getProviderId(provider);
            await storeApiKey(providerId, key);
            setApiKeyInputs(prev => ({ ...prev, [provider]: '' }));
            setValidationStatus(prev => ({ ...prev, [provider]: 'idle' }));
        } catch (error) {
            console.error('Failed to save API key:', error);
        } finally {
            setSavingStatus(prev => ({ ...prev, [provider]: false }));
        }
    };

    const handleDeleteKey = async (provider: ProviderType) => {
        const providerId = getProviderId(provider);
        await deleteApiKey(providerId);
    };

    const toggleShowKey = (provider: ProviderType) => {
        setShowKeys(prev => ({ ...prev, [provider]: !prev[provider] }));
    };

    return (
        <Modal isOpen={isOpen} onOpenChange={(open) => !open && onClose()}>
            <Modal.Backdrop variant="blur">
                <Modal.Container>
                    <Modal.Dialog className="max-w-lg">
                        <Modal.CloseTrigger />
                        <Modal.Header>
                            <Modal.Heading className="flex items-center gap-2">
                                <Settings className="size-5" />
                                Settings
                            </Modal.Heading>
                        </Modal.Header>
                        <Modal.Body className="space-y-4">
                            <Tabs
                                selectedKey={activeTab}
                                onSelectionChange={(key) => setActiveTab(key as string)}
                            >
                                <Tabs.List>
                                    <Tabs.Tab id="providers">
                                        <Key className="size-4" />
                                        AI Providers
                                    </Tabs.Tab>
                                    <Tabs.Tab id="general">
                                        <Zap className="size-4" />
                                        General
                                    </Tabs.Tab>
                                </Tabs.List>

                                <Tabs.Panel id="providers" className="pt-4 space-y-4">
                                    {AI_PROVIDERS.map((provider) => {
                                        const providerId = getProviderId(provider.id);
                                        const hasKey = hasApiKey(providerId);
                                        const status = validationStatus[provider.id] || 'idle';
                                        const isSaving = savingStatus[provider.id];
                                        const showKey = showKeys[provider.id];

                                        return (
                                            <Card key={provider.id} className="p-4">
                                                <div className="space-y-3">
                                                    <div className="flex items-center justify-between">
                                                        <div className="flex items-center gap-2">
                                                            <Sparkles className="size-4 text-primary" />
                                                            <span className="font-medium">{provider.name}</span>
                                                        </div>
                                                        {hasKey && (
                                                            <span className="text-xs text-green-600 flex items-center gap-1">
                                                                <Check className="size-3" />
                                                                Connected
                                                            </span>
                                                        )}
                                                    </div>

                                                    <p className="text-sm text-muted-foreground">
                                                        {provider.description}
                                                    </p>

                                                    {!hasKey ? (
                                                        <div className="space-y-2">
                                                            <div className="flex gap-2">
                                                                <TextField
                                                                    className="flex-1"
                                                                    name={`api-key-${provider.id}`}
                                                                    type={showKey ? 'text' : 'password'}
                                                                    onChange={(value) => handleApiKeyChange(provider.id, value)}
                                                                >
                                                                    <Input
                                                                        placeholder="Enter API key..."
                                                                        value={apiKeyInputs[provider.id] || ''}
                                                                    />
                                                                </TextField>
                                                                <Button
                                                                    variant="secondary"
                                                                    size="sm"
                                                                    onPress={() => toggleShowKey(provider.id)}
                                                                >
                                                                    {showKey ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                                                                </Button>
                                                            </div>
                                                            <div className="flex gap-2">
                                                                <Button
                                                                    variant="secondary"
                                                                    size="sm"
                                                                    onPress={() => handleValidateKey(provider.id)}
                                                                    isDisabled={!apiKeyInputs[provider.id]?.trim() || status === 'validating'}
                                                                >
                                                                    {status === 'validating' ? 'Validating...' : 'Validate'}
                                                                </Button>
                                                                <Button
                                                                    variant="primary"
                                                                    size="sm"
                                                                    onPress={() => handleSaveKey(provider.id)}
                                                                    isDisabled={!apiKeyInputs[provider.id]?.trim() || isSaving}
                                                                >
                                                                    {isSaving ? 'Saving...' : 'Save to Keychain'}
                                                                </Button>
                                                            </div>
                                                            {status === 'valid' && (
                                                                <p className="text-xs text-green-600 flex items-center gap-1">
                                                                    <Check className="size-3" /> API key is valid
                                                                </p>
                                                            )}
                                                            {status === 'invalid' && (
                                                                <p className="text-xs text-red-600 flex items-center gap-1">
                                                                    <X className="size-3" /> Invalid API key
                                                                </p>
                                                            )}
                                                        </div>
                                                    ) : (
                                                        <div className="flex items-center justify-between">
                                                            <span className="text-sm text-muted-foreground">
                                                                API key stored in Keychain
                                                            </span>
                                                            <Button
                                                                variant="secondary"
                                                                size="sm"
                                                                onPress={() => handleDeleteKey(provider.id)}
                                                            >
                                                                Remove
                                                            </Button>
                                                        </div>
                                                    )}
                                                </div>
                                            </Card>
                                        );
                                    })}
                                </Tabs.Panel>

                                <Tabs.Panel id="general" className="pt-4 space-y-4">
                                    <div className="space-y-3">
                                        <div className="flex items-center justify-between">
                                            <div>
                                                <Label className="font-medium">Notifications</Label>
                                                <p className="text-sm text-muted-foreground">Show task completion alerts</p>
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
                                                <Label className="font-medium">Auto-execute tasks</Label>
                                                <p className="text-sm text-muted-foreground">Start tasks immediately after creation</p>
                                            </div>
                                            <Switch>
                                                <Switch.Control>
                                                    <Switch.Thumb />
                                                </Switch.Control>
                                            </Switch>
                                        </div>
                                    </div>
                                </Tabs.Panel>
                            </Tabs>
                        </Modal.Body>
                        <Modal.Footer>
                            <Button variant="secondary" onPress={onClose}>
                                Close
                            </Button>
                        </Modal.Footer>
                    </Modal.Dialog>
                </Modal.Container>
            </Modal.Backdrop>
        </Modal>
    );
}

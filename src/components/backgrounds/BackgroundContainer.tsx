
import { ReactNode } from 'react';

export const BackgroundContainer = ({ children }: { children: ReactNode }) => {
    return (
        <div className="fixed inset-0 -z-50 overflow-hidden pointer-events-none select-none">
            {children}
        </div>
    );
};


import { memo } from 'react';

export const JujutsuKaisenBg = memo(() => {
    return (
        <>
            {/* Cursed Energy Overlay - Red (Sukuna) */}
            <div
                className="absolute top-[-20%] left-[-10%] w-[70vw] h-[70vw] rounded-full blur-[120px] opacity-20 animate-pulse-slow"
                style={{
                    background: 'radial-gradient(circle, var(--accent) 0%, transparent 70%)',
                    animationDuration: '10s'
                }}
            />

            {/* Cursed Energy Overlay - Blue (Gojo) */}
            <div
                className="absolute bottom-[-10%] right-[-10%] w-[60vw] h-[60vw] rounded-full blur-[100px] opacity-15 animate-float"
                style={{
                    background: 'radial-gradient(circle, var(--primary) 0%, transparent 70%)',
                    animationDuration: '15s',
                    animationDelay: '1s'
                }}
            />

            {/* Void Texture */}
            <div className="absolute inset-0 opacity-[0.03]"
                style={{
                    backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")`
                }}
            />
        </>
    );
});

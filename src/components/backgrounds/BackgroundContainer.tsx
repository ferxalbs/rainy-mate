import { ReactNode } from "react";

export const BackgroundContainer = ({ children }: { children: ReactNode }) => {
  return (
    <div className="absolute inset-0 z-0 overflow-hidden pointer-events-none select-none">
      {children}
    </div>
  );
};

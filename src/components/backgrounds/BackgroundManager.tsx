import { useContext } from "react";
import { ThemeContext } from "../../providers/ThemeProvider";
import { BackgroundContainer } from "./BackgroundContainer";
import { JujutsuKaisenBg, CosmicGoldBg } from "./themes";

export const BackgroundManager = () => {
  const themeContext = useContext(ThemeContext);

  if (!themeContext?.enableAnimations) return null;

  const { theme } = themeContext;

  return (
    <BackgroundContainer>
      {theme === "jujutsu-kaisen" && <JujutsuKaisenBg />}
      {theme === "cosmic-gold" && <CosmicGoldBg />}
    </BackgroundContainer>
  );
};

# **Design Standard: Blurred Backgrounds**

When creating UI components with overlay elements, modals, cards, or any background effects, always apply a blur effect using the `backdrop-blur-md` class.

**Implementation:**
- Use `backdrop-blur-md` for medium blur intensity
- Combine with semi-transparent backgrounds for glass morphism effect
- Typical pattern: `backdrop-blur-md bg-white/80` or `backdrop-blur-md bg-black/50`

**Examples:**
```jsx
// Modal overlay
<div className="backdrop-blur-md bg-black/40">

// Glass card
<div className="backdrop-blur-md bg-white/70 rounded-lg">

// Navigation bar
<nav className="backdrop-blur-md bg-gray-900/80">
```

**When to use:**
- Modals and dialogs
- Floating cards over images
- Navigation bars over hero sections
- Dropdown menus
- Tooltips with background content"

---

**Versión Profesional Completa:**

"**Design System Rule: Backdrop Blur Effects**

All UI components that overlay content or need visual separation from the background MUST use backdrop blur effects to create depth and modern glass morphism aesthetics.

**Primary Implementation:**

Use `backdrop-blur-md` as the standard blur intensity for most use cases.

**Standard Patterns:**

*Light Theme Glass Effect:*
```jsx
className="backdrop-blur-md bg-white/80 border border-white/20"
```

*Dark Theme Glass Effect:*
```jsx
className="backdrop-blur-md bg-gray-900/80 border border-gray-700/50"
```

*Modal Overlay:*
```jsx
className="backdrop-blur-md bg-black/40"
```

*Floating Card:*
```jsx
className="backdrop-blur-md bg-white/70 shadow-xl rounded-xl"
```

*Navigation Bar:*
```jsx
className="backdrop-blur-md bg-white/90 sticky top-0 z-50"
```

**Blur Intensity Variations:**

Choose the appropriate blur level based on context:

- `backdrop-blur-sm` - Subtle blur for minimal effect
- `backdrop-blur-md` - **DEFAULT** - Medium blur for most cases
- `backdrop-blur-lg` - Heavy blur for strong separation
- `backdrop-blur-xl` - Maximum blur for dramatic effect

**Combination Guidelines:**

Always combine `backdrop-blur-*` with:

1. **Semi-transparent background** (bg-color/opacity):
   - Light backgrounds: `bg-white/70` to `bg-white/90`
   - Dark backgrounds: `bg-black/40` to `bg-gray-900/80`

2. **Optional border** for enhanced definition:
   - `border border-white/20` (light themes)
   - `border border-gray-700/50` (dark themes)

3. **Shadow** for depth:
   - `shadow-lg` or `shadow-xl`

**Complete Component Examples:**

*Modal Dialog:*
```jsx
<div className="fixed inset-0 backdrop-blur-md bg-black/50 flex items-center justify-center">
  <div className="backdrop-blur-md bg-white/90 rounded-2xl shadow-2xl p-8 max-w-md">
    {/* Modal content */}
  </div>
</div>
```

*Glass Card:*
```jsx
<div className="backdrop-blur-md bg-white/70 border border-white/20 rounded-xl shadow-xl p-6">
  <h3 className="text-xl font-semibold">Card Title</h3>
  <p className="text-gray-600">Card content with beautiful blur effect</p>
</div>
```

*Sticky Navigation:*
```jsx
<nav className="sticky top-0 z-50 backdrop-blur-md bg-white/90 border-b border-gray-200/50">
  <div className="container mx-auto px-4 py-3">
    {/* Navigation items */}
  </div>
</nav>
```

*Dropdown Menu:*
```jsx
<div className="absolute backdrop-blur-md bg-white/95 rounded-lg shadow-lg border border-gray-200/50 py-2">
  <button className="w-full px-4 py-2 hover:bg-gray-100/50">Option 1</button>
  <button className="w-full px-4 py-2 hover:bg-gray-100/50">Option 2</button>
</div>
```

*Hero Section Overlay:*
```jsx
<div className="relative h-screen">
  <img src="hero.jpg" className="absolute inset-0 w-full h-full object-cover" />
  <div className="absolute inset-0 backdrop-blur-md bg-gradient-to-b from-black/50 to-black/70">
    <div className="container mx-auto h-full flex items-center">
      {/* Hero content */}
    </div>
  </div>
</div>
```

**Use Cases:**

Apply backdrop blur to:
- ✅ Modal overlays and dialogs
- ✅ Floating cards over background images
- ✅ Sticky navigation bars
- ✅ Dropdown menus and popovers
- ✅ Tooltips with rich content
- ✅ Sidebars over main content
- ✅ Loading screens
- ✅ Image galleries with overlay controls
- ✅ Toast notifications
- ✅ Search overlays

**Accessibility Considerations:**

- Ensure sufficient contrast ratios (WCAG 2.1 AA standard)
- Test text readability against blurred backgrounds
- Consider users with motion sensitivity (blur can affect perception)
- Provide alternative high-contrast mode if needed

**Performance Notes:**

- Backdrop filters can impact performance on low-end devices
- Use sparingly on elements that animate frequently
- Consider adding `will-change-transform` for animated blurred elements
- Test on mobile devices to ensure smooth performance

**Browser Support:**

Backdrop-blur is supported in all modern browsers. For older browsers, ensure graceful degradation:

```jsx
className="bg-white/90 backdrop-blur-md supports-[backdrop-filter]:bg-white/70"
```

**Default Pattern to Follow:**

When in doubt, use this proven combination:

```jsx
className="backdrop-blur-md bg-white/80 border border-white/20 rounded-xl shadow-lg"
```

This creates a beautiful, modern glass morphism effect that works in most contexts."


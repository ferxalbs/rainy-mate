# Rainy Cowork Roadmap

## Current Version: v0.3.0

### v0.3.0 - Phase 3: Advanced Features ✅

**Content Extraction (Complete)**

- [x] URL content extraction (Rust-native HTML→Markdown)
- [x] Content caching with DashMap
- [x] TypeScript hooks for frontend

**Tavily Web Search (Complete)**

- [x] Tavily SDK integration in rainy-api-v2
- [x] Search endpoint: POST /api/v1/search
- [x] Extract endpoint: POST /api/v1/search/extract
- [x] Cowork plan gating for premium feature

**Document Generation (In Progress)**

- [ ] Template-based document generation (Rust)
- [ ] Markdown export with handlebars templates
- [ ] AI-assisted document creation (Premium/Cowork)

**Image Processing (Planned)**

- [ ] Metadata extraction (EXIF, dimensions)
- [ ] Thumbnail generation
- [ ] AI vision analysis (Premium/Cowork)

**macOS Deep Integration (Planned)**

- [ ] Menu bar quick access
- [ ] Shortcuts app integration

---

### v0.3.5 - Email & Calendar Integration (Planned)

> ⚠️ Requires Google OAuth verification

- [ ] OAuth2 authentication flow
- [ ] Gmail reading and composition
- [ ] Calendar event management

---

### v0.5.0 - Plugin Ecosystem (Future)

- [ ] Plugin API definition
- [ ] Community plugin marketplace
- [ ] Custom workflow automation

---

## Open Core Model

| Feature | Free (OSS) | Premium (Cowork) |
|---------|------------|------------------|
| URL Content Extraction | ✅ | ✅ |
| **Web Search (Tavily)** | ❌ | ✅ |
| Document Templates | ✅ | ✅ |
| Image Metadata | ✅ | ✅ |
| AI Document Generation | ❌ | ✅ |
| AI Vision Analysis | ❌ | ✅ |
| Email/Calendar | ❌ | ✅ |

---

*Last Updated: January 2026*

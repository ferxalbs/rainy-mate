---
trigger: always_on
---

"Always use pnpm as the package manager for this project. Do not use npm under any circumstances.

Requirements:

- Use pnpm install for installing dependencies
- Use pnpm add <package> for adding new packages
- Use pnpm remove <package> for removing packages
- Use pnpm run <script> for executing scripts
- Ensure all commands and documentation reference pnpm, not npm

Reasons for using pnpm:

- Faster installation times through efficient disk space usage
- Strict dependency resolution that prevents phantom dependencies
- Better monorepo support
- Reduced disk space consumption with content-addressable storage"
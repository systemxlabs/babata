# Babata

## System prompts
System prompts are stored in ~/.babata/system_prompts folder. All markdown files will be automatically loaded.

## Skills
Skills are stored in ~/.babata/skills folder.

### Disable skill
Add `enable: false` in skill.md yaml section.
```
---
name: pdf-processing
description: Extract text and tables from PDF files, fill forms, merge documents.
enable: false
---
```

### Inline skill
Add `inline: true` in skill.md yaml section.
```
---
name: pdf-processing
description: Extract text and tables from PDF files, fill forms, merge documents.
inline: true
---
```

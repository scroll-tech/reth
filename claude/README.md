# Claude Instructions and Tools

This directory contains instructions and automation tools for AI-assisted development workflows in the Reth repository.

## Contents

### 📄 Instructions

- **`grafana-dashboard-sync-instructions.md`** - Complete guide for synchronizing Grafana dashboards from upstream to Scroll's Kubernetes-customized versions

### 🛠️ Tools

Located in `tools/` directory:

- **`sync_dashboard.py`** - Automated dashboard synchronization script
- **`compare_dashboards.py`** - Dashboard comparison and analysis tool
- **`detailed_dashboard_analysis.py`** - Detailed query-level analysis tool

## Usage

### Grafana Dashboard Synchronization

To sync Grafana dashboards with upstream:

```bash
# From repository root
python3 claude/tools/compare_dashboards.py > comparison.txt
python3 claude/tools/sync_dashboard.py
```

See `grafana-dashboard-sync-instructions.md` for complete documentation.

## Purpose

This directory serves as a knowledge base for repeatable AI-assisted workflows. When working with AI assistants (like Claude), these instructions help ensure consistent and correct execution of complex tasks.

## Contributing

When adding new workflows:

1. Create a detailed instruction file (`.md`)
2. Add any supporting scripts to `tools/`
3. Update this README with a brief description
4. Include usage examples and prerequisites

---

**Last updated:** 2025-12-01

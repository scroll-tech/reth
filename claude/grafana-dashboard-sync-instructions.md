# Grafana Dashboard Synchronization Instructions

## Overview

This document provides instructions for synchronizing Scroll's Kubernetes-customized Grafana dashboards with upstream reth dashboards. This process should be performed periodically when upstream updates are released.

## Directory Structure

- **Upstream dashboards:** `etc/grafana/dashboards/` (canonical source)
- **Scroll K8s dashboards:** `etc/grafana/scroll/` (customized versions)

## When to Sync

Sync dashboards when:
- Upstream reth releases new dashboard versions
- New monitoring features are added upstream
- Structural changes are made to upstream dashboards
- Every 1-3 months as part of regular maintenance

## Synchronization Process

### Step 1: Analyze Differences

First, understand what has changed upstream:

```bash
# Run the comparison script to identify changes
python3 claude/tools/compare_dashboards.py > comparison_report.txt

# Review the report
cat comparison_report.txt
```

The comparison will show:
- **Case A:** Dashboards that exist in both directories (need sync)
- **Case B:** New dashboards in upstream only (evaluate for porting)
- **Case C:** Dashboards only in scroll (evaluate for retention)

### Step 2: Execute Synchronization

Run the automated sync script:

```bash
# Execute the sync script
python3 claude/tools/sync_dashboard.py
```

This script will:
- Use upstream dashboards as the base structure
- Add K8s variables (env, service) to all dashboards - NO pod variable
- Preserve dashboard-specific variables (e.g., interval) alongside K8s variables
- Transform all PromQL queries to use service-only label selectors
- Hardcode datasource UID `o59qe-zVz` in all panels
- Preserve Scroll UIDs
- Save updated dashboards to `etc/grafana/scroll/`

### Step 3: Validate Changes

Verify the synchronization was successful:

```bash
# Validate JSON syntax
python3 -c "
import json
from pathlib import Path

for f in Path('etc/grafana/scroll').glob('*.json'):
    with open(f) as fp:
        data = json.load(fp)
    print(f'✓ {f.name} - Valid ({len(data.get(\"panels\", []))} panels)')
"

# Review changes
git diff etc/grafana/scroll/
```

### Step 4: Manual Review

Check for:
- **New panels:** Review new upstream panels and verify they work with K8s labels
- **Removed panels:** Check if any scroll-specific panels were lost
- **Query correctness:** Spot-check that K8s label transformations are correct
- **Variable definitions:** Ensure K8s variables are present in all dashboards

### Step 5: Test in Grafana

Before committing:

1. **Deploy to staging/test Grafana instance**
   ```bash
   # Update ConfigMap (adjust for your deployment)
   kubectl create configmap grafana-dashboards \
     --from-file=etc/grafana/scroll/ \
     --dry-run=client -o yaml | kubectl apply -f -

   kubectl rollout restart deployment/grafana -n monitoring
   ```

2. **Test each dashboard:**
   - [ ] Variables populate correctly (env, service) - only 2 variables
   - [ ] All panels display data
   - [ ] No query errors
   - [ ] New panels work as expected
   - [ ] Time ranges and refresh work

3. **Performance check:**
   - [ ] Dashboard load time < 5 seconds
   - [ ] No Prometheus query timeouts
   - [ ] Query execution time acceptable

### Step 6: Commit Changes

If all tests pass:

```bash
# Stage changes
git add etc/grafana/scroll/

# Commit with descriptive message
git commit -m "feat: sync Grafana dashboards with upstream

- Update all dashboards with latest upstream structure
- Add [N] new panels from upstream
- Transform queries to use K8s label selectors
- Preserve Scroll UIDs and K8s customizations

New panels:
- [List notable new panels]

Testing: Verified all queries and variables work in [environment]"

# Push changes
git push
```

## Kubernetes Customization Pattern

### Standard K8s Variables

All Scroll dashboards must include these variables (2 only - NO pod variable):

```json
{
  "name": "env",
  "type": "query",
  "definition": "label_values(env)",
  "query": {
    "qryType": 1,
    "query": "label_values(env)",
    "refId": "PrometheusVariableQueryEditor-VariableQuery"
  },
  "regex": "(sepolia|mainnet)-eks.*"
}

{
  "name": "service",
  "type": "query",
  "definition": "label_values(reth_info{namespace=\"$env\"},service)",
  "query": {
    "qryType": 1,
    "query": "label_values(reth_info{namespace=\"$env\"},service)",
    "refId": "PrometheusVariableQueryEditor-VariableQuery"
  },
  "regex": "(l[1|2]reth.*)"
}
```

**Important:**
- No `pod` variable - queries aggregate by service only, enabling data continuity when pods are replaced
- No `datasource` variable - datasource UID is hardcoded in all panels
- **Dashboard-specific variables are preserved:** Some dashboards have additional variables (e.g., `interval` in reth-state-growth.json) that must be preserved alongside the K8s variables

### Hardcoded Datasource

All panels and targets use a hardcoded Prometheus datasource UID:

```json
{
  "datasource": {
    "type": "prometheus",
    "uid": "o59qe-zVz"
  }
}
```

This matches the Scroll deployment's Prometheus datasource configuration.

### Query Transformation Rules

The sync script applies these transformations:

| Upstream Pattern | Scroll Pattern (K8s) |
|------------------|----------------------|
| `$instance_label="$instance"` | `service="$service", namespace="$env"` |
| `instance="$instance"` | `service="$service", namespace="$env"` |
| `instance=~"$instance"` | `service="$service", namespace="$env"` |

**Important:**
- Uses exact match (`=`) not regex match (`=~`) for precise service filtering
- Includes `namespace="$env"` to prevent cross-environment data aggregation

**Example:**
```promql
# Upstream:
reth_database_operation_duration{$instance_label="$instance", quantile="0.99"}

# Scroll (after transformation):
reth_database_operation_duration{service="$service", namespace="$env", quantile="0.99"}
```

### Data Continuity Feature

By using **service and namespace** filtering (no pod label), dashboards maintain historical data when pods are replaced:
- Old pod dies → new pod starts with different name
- Both pods share the same `service` and `namespace` labels
- Queries aggregate across all pods for that service in that environment
- Historical data remains visible seamlessly
- **Exact match** ensures `service-0` only shows `service-0` data, not `service-1`
- **Namespace filter** prevents cross-environment data mixing (mainnet vs sepolia)

## Handling Special Cases

### New Upstream Dashboards (Case B)

When upstream adds a new dashboard:

1. **Evaluate relevance:**
   - Is it applicable to Scroll's deployment?
   - Does it use metrics available in Scroll's reth build?
   - Is it worth maintaining?

2. **If relevant, port it:**
   - Copy upstream dashboard as base
   - Run sync script or manually add K8s variables
   - Transform all queries
   - Test thoroughly
   - Add to `etc/grafana/scroll/`

3. **If not relevant:**
   - Document decision in git commit
   - Skip porting

### Scroll-Only Dashboards (Case C)

If Scroll has custom dashboards not in upstream:

1. **Check if metrics are now in upstream:**
   - Search upstream dashboards for the same metrics
   - If covered, consider removing custom dashboard

2. **If still unique:**
   - Keep the custom dashboard
   - Ensure it follows K8s variable pattern
   - Document its purpose

### Major Upstream Changes

If upstream significantly refactors dashboard structure:

1. **Backup current scroll versions:**
   ```bash
   cp -r etc/grafana/scroll etc/grafana/scroll.backup.$(date +%Y%m%d)
   ```

2. **Run sync with caution:**
   - Review changes carefully
   - Test extensively before committing
   - Consider phased rollout (one dashboard at a time)

3. **Document breaking changes:**
   - Note in commit message
   - Update team documentation
   - Inform monitoring team

## Troubleshooting

### Issue: Sync script fails with errors

**Solution:**
- Check that upstream dashboards are valid JSON
- Verify Python 3 is available
- Review error message and fix specific issue
- May need to update sync script for new patterns

### Issue: Variables don't populate after sync

**Solution:**
- Verify Prometheus has required labels: `env`, `pod`, `service`, `namespace`
- Check variable query syntax in dashboard JSON
- Test queries directly in Prometheus UI
- Ensure label names match your Helm deployment

### Issue: Queries return no data after transformation

**Solution:**
- Check that K8s label selectors match your deployment
- Verify metric label structure in Prometheus
- Test transformed query directly in Prometheus
- May need to adjust label names in transformation

### Issue: New panels from upstream don't work

**Solution:**
- Verify metrics exist in your reth build version
- Check if feature flags are needed
- Some metrics may be version-specific or feature-specific
- Consider removing panels for unavailable features

## Maintenance Scripts

All scripts are located in `claude/tools/` directory.

### compare_dashboards.py

Compares upstream and scroll dashboards, showing:
- Panel count differences
- Variable differences
- Structural changes
- K8s customization patterns

**Usage:**
```bash
python3 claude/tools/compare_dashboards.py > report.txt
```

### sync_dashboard.py

Automated synchronization script that:
- Loads upstream as base
- Adds K8s variables
- Transforms all queries
- Preserves Scroll UIDs
- Saves to scroll directory

**Usage:**
```bash
python3 claude/tools/sync_dashboard.py
```

**Customization:**
Edit the `dashboards` list in `main()` to add/remove dashboards to sync.

### detailed_dashboard_analysis.py

Generates detailed analysis including:
- Query-by-query comparison
- Specific transformation plans
- Migration guidance

**Usage:**
```bash
python3 claude/tools/detailed_dashboard_analysis.py > analysis.txt
```

## Quick Reference Commands

```bash
# Full sync workflow
python3 claude/tools/compare_dashboards.py > comparison.txt
cat comparison.txt
python3 claude/tools/sync_dashboard.py
git diff etc/grafana/scroll/
# Review, test, commit

# Validate JSON
python3 -m json.tool etc/grafana/scroll/*.json > /dev/null && echo "All valid"

# Check file sizes
ls -lh etc/grafana/scroll/

# Count panels per dashboard
for f in etc/grafana/scroll/*.json; do
  echo "$(basename $f): $(jq '.panels | length' $f) panels"
done

# Extract all metrics used
for f in etc/grafana/scroll/*.json; do
  jq -r '.. | .expr? // empty' $f | grep -oE 'reth_\w+' | sort -u
done
```

## Best Practices

1. **Always backup before major syncs:**
   ```bash
   cp -r etc/grafana/scroll etc/grafana/scroll.backup.$(date +%Y%m%d)
   ```

2. **Test in non-production first:**
   - Deploy to staging/dev Grafana
   - Verify all functionality
   - Get team review

3. **Sync regularly:**
   - Monthly check for upstream changes
   - Don't let versions drift too far

4. **Document custom changes:**
   - If you manually modify dashboards, document why
   - Consider contributing improvements upstream

5. **Keep scripts updated:**
   - Update transformation patterns as needed
   - Add new label patterns if deployment changes

6. **Version control everything:**
   - Commit dashboards to git
   - Use descriptive commit messages
   - Tag releases if using versioned deployments

## Resources

- **Upstream reth repository:** https://github.com/paradigmxyz/reth
- **Grafana documentation:** https://grafana.com/docs/
- **PromQL documentation:** https://prometheus.io/docs/prometheus/latest/querying/basics/

## Contact

For questions about this process:
- Review git history for previous sync commits
- Check `GRAFANA_DASHBOARD_SYNC_PLAN.md` for detailed analysis
- Consult with the monitoring/observability team

---

**Last updated:** 2025-12-01
**Last sync:** 2025-12-01 (Converged with upstream, service-only pattern for data continuity)
**Pattern:**
- 2 variables only: `env`, `service` (NO pod variable)
- Hardcoded datasource UID: `o59qe-zVz`
- Enables seamless pod replacement with data continuity

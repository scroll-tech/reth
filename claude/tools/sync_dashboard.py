#!/usr/bin/env python3
"""
Grafana Dashboard K8s Transformation Script
Syncs upstream dashboard structure with Scroll's Kubernetes customizations
"""

import json
import re
import sys
from pathlib import Path
from typing import Dict, Any, List
from copy import deepcopy

def add_k8s_variables(dashboard: Dict, preserve_uid: str = None) -> Dict:
    """Add standard K8s variables to dashboard templating (env and service only)"""
    k8s_vars = [
        {
            "current": {
                "text": "mainnet",
                "value": "mainnet"
            },
            "definition": "label_values(env)",
            "name": "env",
            "options": [],
            "query": {
                "qryType": 1,
                "query": "label_values(env)",
                "refId": "PrometheusVariableQueryEditor-VariableQuery"
            },
            "refresh": 1,
            "regex": "(sepolia|mainnet)-eks.*",
            "type": "query"
        },
        {
            "current": {
                "text": "l1reth-el-0",
                "value": "l1reth-el-0"
            },
            "definition": "label_values(reth_info{namespace=\"$env\"},service)",
            "name": "service",
            "options": [],
            "query": {
                "qryType": 1,
                "query": "label_values(reth_info{namespace=\"$env\"},service)",
                "refId": "PrometheusVariableQueryEditor-VariableQuery"
            },
            "refresh": 1,
            "regex": "(l[1|2]reth.*)",
            "type": "query"
        }
    ]

    if 'templating' not in dashboard:
        dashboard['templating'] = {'list': []}

    # Preserve dashboard-specific variables (like interval for reth-state-growth)
    existing_vars = dashboard.get('templating', {}).get('list', [])
    preserved_vars = [v for v in existing_vars if v.get('name') in ['interval']]

    # Replace with K8s variables + preserved dashboard-specific variables
    dashboard['templating']['list'] = k8s_vars + preserved_vars

    # Preserve scroll UID if provided
    if preserve_uid:
        dashboard['uid'] = preserve_uid

    return dashboard

def transform_query(query: str) -> str:
    """
    Transform PromQL query to use K8s labels (service and namespace, no pod)
    Uses exact match (=) not regex (=~) for precise service filtering
    Includes namespace filter to prevent cross-environment aggregation
    This enables data continuity when pods are replaced
    """
    if not query or not isinstance(query, str):
        return query

    # Pattern 1: $instance_label="$instance" or $instance_label=~"$instance"
    query = re.sub(
        r'\$instance_label\s*=~?\s*["\']?\$instance["\']?',
        'service="$service", namespace="$env"',
        query
    )

    # Pattern 2: instance="$instance" or instance=~"$instance" (direct usage)
    query = re.sub(
        r'instance\s*=~?\s*["\']?\$instance["\']?',
        'service="$service", namespace="$env"',
        query
    )

    # Pattern 3: {$instance_label="$instance"} at start of label set
    query = re.sub(
        r'\{\s*\$instance_label\s*=~?\s*["\']?\$instance["\']?\s*,',
        '{service="$service", namespace="$env",',
        query
    )

    # Pattern 4: {instance="$instance"} at start of label set
    query = re.sub(
        r'\{\s*instance\s*=~?\s*["\']?\$instance["\']?\s*,',
        '{service="$service", namespace="$env",',
        query
    )

    # Pattern 5: , $instance_label="$instance"} at end of label set
    query = re.sub(
        r',\s*\$instance_label\s*=~?\s*["\']?\$instance["\']?\s*\}',
        ', service="$service", namespace="$env"}',
        query
    )

    # Pattern 6: , instance="$instance"} at end of label set
    query = re.sub(
        r',\s*instance\s*=~?\s*["\']?\$instance["\']?\s*\}',
        ', service="$service", namespace="$env"}',
        query
    )

    # Pattern 7: {$instance_label="$instance"} as only label
    query = re.sub(
        r'\{\s*\$instance_label\s*=~?\s*["\']?\$instance["\']?\s*\}',
        '{service="$service", namespace="$env"}',
        query
    )

    # Pattern 8: {instance="$instance"} as only label
    query = re.sub(
        r'\{\s*instance\s*=~?\s*["\']?\$instance["\']?\s*\}',
        '{service="$service", namespace="$env"}',
        query
    )

    return query

def transform_target(target: Dict) -> Dict:
    """Transform a single query target"""
    if 'expr' in target and target['expr']:
        target['expr'] = transform_query(target['expr'])
    return target

def set_hardcoded_datasource(obj: Any) -> Any:
    """Replace all datasource references with hardcoded UID"""
    if isinstance(obj, dict):
        # If this is a datasource object, replace with hardcoded UID
        if 'datasource' in obj:
            obj['datasource'] = {
                "type": "prometheus",
                "uid": "o59qe-zVz"
            }
        # Recursively process all dict values
        for key, value in obj.items():
            obj[key] = set_hardcoded_datasource(value)
    elif isinstance(obj, list):
        # Recursively process all list items
        return [set_hardcoded_datasource(item) for item in obj]

    return obj

def transform_panel(panel: Dict) -> Dict:
    """Transform all queries in a panel recursively"""
    # Transform targets in this panel
    if 'targets' in panel:
        panel['targets'] = [transform_target(t) for t in panel['targets']]

    # Set hardcoded datasource for panel and all nested objects
    panel = set_hardcoded_datasource(panel)

    # Recursively handle nested panels (rows with collapsed panels)
    if 'panels' in panel:
        panel['panels'] = [transform_panel(p) for p in panel['panels']]

    return panel

def sync_dashboard(upstream_path: str, scroll_uid: str = None, output_path: str = None) -> Dict:
    """
    Main sync function: takes upstream dashboard and applies K8s transformations

    Args:
        upstream_path: Path to upstream dashboard JSON
        scroll_uid: UID to preserve from scroll version (optional)
        output_path: Where to save the result (optional, defaults to print)

    Returns:
        Transformed dashboard dict
    """
    # Load upstream dashboard
    with open(upstream_path, 'r') as f:
        dashboard = json.load(f)

    print(f"Processing: {dashboard.get('title', 'Unknown')}")
    print(f"  Upstream panels: {len(dashboard.get('panels', []))}")

    # Add K8s variables
    dashboard = add_k8s_variables(dashboard, preserve_uid=scroll_uid)

    # Transform all panels
    panel_count = 0
    target_count = 0

    transformed_panels = []
    for panel in dashboard.get('panels', []):
        panel = transform_panel(panel)
        transformed_panels.append(panel)
        panel_count += 1
        if 'targets' in panel:
            target_count += len(panel['targets'])
        if 'panels' in panel:  # Row with nested panels
            for subpanel in panel['panels']:
                panel_count += 1
                if 'targets' in subpanel:
                    target_count += len(subpanel['targets'])

    dashboard['panels'] = transformed_panels

    print(f"  Transformed panels: {panel_count}")
    print(f"  Transformed queries: {target_count}")
    print(f"  Variables: {len(dashboard['templating']['list'])}")

    # Save if output path provided
    if output_path:
        with open(output_path, 'w') as f:
            json.dump(dashboard, f, indent=2)
        print(f"  ✓ Saved to: {output_path}")

    return dashboard

def get_scroll_uid(scroll_path: str) -> str:
    """Extract UID from existing scroll dashboard"""
    try:
        with open(scroll_path, 'r') as f:
            data = json.load(f)
            return data.get('uid')
    except:
        return None

def main():
    """Process all dashboards"""
    upstream_dir = Path('etc/grafana/dashboards')
    scroll_dir = Path('etc/grafana/scroll')

    # Dashboards to sync
    dashboards = [
        'overview.json',
        'reth-discovery.json',
        'reth-mempool.json',
        'reth-state-growth.json',
    ]

    print("=" * 80)
    print("GRAFANA DASHBOARD SYNCHRONIZATION")
    print("=" * 80)
    print()

    for filename in dashboards:
        upstream_path = upstream_dir / filename
        scroll_path = scroll_dir / filename
        output_path = scroll_dir / filename

        # Get scroll UID to preserve
        scroll_uid = get_scroll_uid(scroll_path) if scroll_path.exists() else None

        print(f"\n{'=' * 80}")
        print(f"Dashboard: {filename}")
        if scroll_uid:
            print(f"  Preserving UID: {scroll_uid}")
        print(f"{'=' * 80}")

        # Sync and save
        sync_dashboard(str(upstream_path), scroll_uid, str(output_path))

    print("\n" + "=" * 80)
    print("SYNCHRONIZATION COMPLETE")
    print("=" * 80)
    print("\nNext steps:")
    print("  1. Review the updated dashboards")
    print("  2. Validate JSON syntax")
    print("  3. Test in Grafana")
    print("  4. Commit changes")

if __name__ == '__main__':
    main()

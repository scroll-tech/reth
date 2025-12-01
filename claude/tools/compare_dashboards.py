#!/usr/bin/env python3
"""
Grafana Dashboard Comparison Tool
Compares upstream and scroll-customized dashboards
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Set, Any
from collections import defaultdict

def load_json(filepath: Path) -> Dict:
    """Load JSON dashboard file"""
    with open(filepath, 'r') as f:
        return json.load(f)

def extract_panel_info(panel: Dict) -> Dict:
    """Extract key information from a panel"""
    return {
        'id': panel.get('id'),
        'title': panel.get('title', 'N/A'),
        'type': panel.get('type'),
        'datasource': panel.get('datasource'),
        'gridPos': panel.get('gridPos'),
        'targets_count': len(panel.get('targets', [])),
        'targets': [
            {
                'expr': t.get('expr', 'N/A')[:100],  # Truncate long queries
                'legendFormat': t.get('legendFormat'),
            }
            for t in panel.get('targets', [])
        ]
    }

def extract_template_vars(dashboard: Dict) -> List[Dict]:
    """Extract template variables"""
    templating = dashboard.get('templating', {})
    variables = templating.get('list', [])
    return [
        {
            'name': v.get('name'),
            'type': v.get('type'),
            'query': str(v.get('query', ''))[:100],
            'datasource': v.get('datasource'),
        }
        for v in variables
    ]

def extract_dashboard_structure(filepath: Path) -> Dict:
    """Extract key structural elements from dashboard"""
    data = load_json(filepath)

    panels = []
    for panel in data.get('panels', []):
        if panel.get('type') == 'row':
            # Row with nested panels
            panels.append({
                'type': 'row',
                'title': panel.get('title'),
                'collapsed': panel.get('collapsed', False),
            })
            for subpanel in panel.get('panels', []):
                panels.append(extract_panel_info(subpanel))
        else:
            panels.append(extract_panel_info(panel))

    return {
        'title': data.get('title'),
        'uid': data.get('uid'),
        'version': data.get('version'),
        'panels_count': len(panels),
        'panels': panels,
        'variables': extract_template_vars(data),
        'variables_count': len(extract_template_vars(data)),
        'refresh': data.get('refresh'),
        'time': data.get('time'),
    }

def find_kubernetes_patterns(filepath: Path) -> Dict[str, List[str]]:
    """Find Kubernetes-specific patterns in the dashboard"""
    with open(filepath, 'r') as f:
        content = f.read()

    patterns = {
        'namespace_selectors': [],
        'pod_selectors': [],
        'job_selectors': [],
        'release_selectors': [],
        'k8s_labels': [],
    }

    # Search for common Kubernetes label patterns
    import re

    # Find namespace patterns
    namespace_matches = re.findall(r'namespace=~?"([^"]+)"', content)
    patterns['namespace_selectors'] = list(set(namespace_matches))

    # Find pod patterns
    pod_matches = re.findall(r'pod=~?"([^"]+)"', content)
    patterns['pod_selectors'] = list(set(pod_matches))

    # Find job patterns
    job_matches = re.findall(r'job=~?"([^"]+)"', content)
    patterns['job_selectors'] = list(set(job_matches))

    # Find release patterns
    release_matches = re.findall(r'release=~?"([^"]+)"', content)
    patterns['release_selectors'] = list(set(release_matches))

    # Find variable references
    var_matches = re.findall(r'\$(\w+)', content)
    patterns['k8s_labels'] = [v for v in set(var_matches) if v in ['namespace', 'pod', 'job', 'release', 'instance']]

    return patterns

def compare_panels(upstream_panels: List[Dict], scroll_panels: List[Dict]) -> Dict:
    """Compare panel structures between upstream and scroll"""
    upstream_titles = {p.get('title'): p for p in upstream_panels if p.get('title')}
    scroll_titles = {p.get('title'): p for p in scroll_panels if p.get('title')}

    common_titles = set(upstream_titles.keys()) & set(scroll_titles.keys())
    only_upstream = set(upstream_titles.keys()) - set(scroll_titles.keys())
    only_scroll = set(scroll_titles.keys()) - set(upstream_titles.keys())

    query_diffs = []
    for title in common_titles:
        up = upstream_titles[title]
        sc = scroll_titles[title]

        if up.get('targets') != sc.get('targets'):
            query_diffs.append({
                'title': title,
                'upstream_targets': up.get('targets'),
                'scroll_targets': sc.get('targets'),
            })

    return {
        'common_panels': len(common_titles),
        'only_upstream': sorted(list(only_upstream)),
        'only_scroll': sorted(list(only_scroll)),
        'query_differences': query_diffs,
    }

def compare_variables(upstream_vars: List[Dict], scroll_vars: List[Dict]) -> Dict:
    """Compare template variables"""
    upstream_names = {v['name']: v for v in upstream_vars}
    scroll_names = {v['name']: v for v in scroll_vars}

    common = set(upstream_names.keys()) & set(scroll_names.keys())
    only_upstream = set(upstream_names.keys()) - set(scroll_names.keys())
    only_scroll = set(scroll_names.keys()) - set(upstream_names.keys())

    differences = []
    for name in common:
        if upstream_names[name] != scroll_names[name]:
            differences.append({
                'name': name,
                'upstream': upstream_names[name],
                'scroll': scroll_names[name],
            })

    return {
        'common_variables': len(common),
        'only_upstream': sorted(list(only_upstream)),
        'only_scroll': sorted(list(only_scroll)),
        'modified': differences,
    }

def main():
    upstream_dir = Path('etc/grafana/dashboards')
    scroll_dir = Path('etc/grafana/scroll')

    # Find all dashboards
    upstream_files = {f.name: f for f in upstream_dir.glob('*.json')}
    scroll_files = {f.name: f for f in scroll_dir.glob('*.json')}

    print("=" * 80)
    print("GRAFANA DASHBOARD COMPARISON REPORT")
    print("=" * 80)
    print()

    # Case A: Dashboards in both directories
    common_files = set(upstream_files.keys()) & set(scroll_files.keys())
    print(f"CASE A: Dashboards in both directories ({len(common_files)})")
    print("-" * 80)

    for filename in sorted(common_files):
        print(f"\n### {filename}")
        print()

        upstream_path = upstream_files[filename]
        scroll_path = scroll_files[filename]

        # Extract structures
        upstream_struct = extract_dashboard_structure(upstream_path)
        scroll_struct = extract_dashboard_structure(scroll_path)

        # Find K8s patterns
        k8s_patterns = find_kubernetes_patterns(scroll_path)

        print(f"Title: {upstream_struct['title']}")
        print(f"Upstream UID: {upstream_struct['uid']}")
        print(f"Scroll UID: {scroll_struct['uid']}")
        print()

        print(f"Panels: Upstream={upstream_struct['panels_count']}, Scroll={scroll_struct['panels_count']}")
        print(f"Variables: Upstream={upstream_struct['variables_count']}, Scroll={scroll_struct['variables_count']}")
        print()

        # Compare panels
        panel_comparison = compare_panels(upstream_struct['panels'], scroll_struct['panels'])
        print(f"Panel Analysis:")
        print(f"  - Common panels: {panel_comparison['common_panels']}")
        if panel_comparison['only_upstream']:
            print(f"  - Only in upstream: {panel_comparison['only_upstream']}")
        if panel_comparison['only_scroll']:
            print(f"  - Only in scroll: {panel_comparison['only_scroll']}")
        if panel_comparison['query_differences']:
            print(f"  - Panels with query differences: {len(panel_comparison['query_differences'])}")
        print()

        # Compare variables
        var_comparison = compare_variables(upstream_struct['variables'], scroll_struct['variables'])
        print(f"Variable Analysis:")
        print(f"  - Common variables: {var_comparison['common_variables']}")
        if var_comparison['only_upstream']:
            print(f"  - Only in upstream: {var_comparison['only_upstream']}")
        if var_comparison['only_scroll']:
            print(f"  - Only in scroll: {var_comparison['only_scroll']}")
        if var_comparison['modified']:
            print(f"  - Modified variables: {[v['name'] for v in var_comparison['modified']]}")
        print()

        # Show K8s patterns
        print("Kubernetes-specific patterns in scroll version:")
        for key, values in k8s_patterns.items():
            if values:
                print(f"  - {key}: {values}")
        print()

        print("-" * 80)

    # Case B: Only in upstream
    only_upstream = set(upstream_files.keys()) - set(scroll_files.keys())
    print(f"\nCASE B: Dashboards only in upstream ({len(only_upstream)})")
    print("-" * 80)
    for filename in sorted(only_upstream):
        upstream_path = upstream_files[filename]
        upstream_struct = extract_dashboard_structure(upstream_path)
        print(f"\n### {filename}")
        print(f"Title: {upstream_struct['title']}")
        print(f"Panels: {upstream_struct['panels_count']}")
        print(f"Variables: {upstream_struct['variables_count']}")
        print()

    # Case C: Only in scroll
    only_scroll = set(scroll_files.keys()) - set(upstream_files.keys())
    print(f"\nCASE C: Dashboards only in scroll ({len(only_scroll)})")
    print("-" * 80)
    for filename in sorted(only_scroll):
        scroll_path = scroll_files[filename]
        scroll_struct = extract_dashboard_structure(scroll_path)
        k8s_patterns = find_kubernetes_patterns(scroll_path)

        print(f"\n### {filename}")
        print(f"Title: {scroll_struct['title']}")
        print(f"Panels: {scroll_struct['panels_count']}")
        print(f"Variables: {scroll_struct['variables_count']}")
        print("\nKubernetes-specific patterns:")
        for key, values in k8s_patterns.items():
            if values:
                print(f"  - {key}: {values}")
        print()

if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""
Detailed Grafana Dashboard Analysis
Provides specific migration and update plans for each dashboard
"""

import json
from pathlib import Path
from typing import Dict, List, Set, Any
import re

def load_json(filepath: Path) -> Dict:
    """Load JSON dashboard file"""
    with open(filepath, 'r') as f:
        return json.load(f)

def extract_all_queries(dashboard: Dict) -> List[Dict]:
    """Extract all PromQL queries from dashboard"""
    queries = []

    def process_panel(panel: Dict, parent_title: str = None):
        title = panel.get('title', 'Untitled')
        if parent_title:
            title = f"{parent_title} > {title}"

        for target in panel.get('targets', []):
            if 'expr' in target:
                queries.append({
                    'panel': title,
                    'expr': target['expr'],
                    'legendFormat': target.get('legendFormat', ''),
                    'refId': target.get('refId', ''),
                })

    for panel in dashboard.get('panels', []):
        if panel.get('type') == 'row':
            row_title = panel.get('title')
            for subpanel in panel.get('panels', []):
                process_panel(subpanel, row_title)
        else:
            process_panel(panel)

    return queries

def analyze_query_patterns(query: str) -> Dict[str, List[str]]:
    """Analyze PromQL query for label patterns"""
    patterns = {
        'label_filters': [],
        'variables': [],
        'functions': [],
        'metrics': [],
    }

    # Extract label filters
    label_matches = re.findall(r'(\w+)=~?"([^"]+)"', query)
    patterns['label_filters'] = [(k, v) for k, v in label_matches]

    # Extract variables
    var_matches = re.findall(r'\$(\w+)', query)
    patterns['variables'] = list(set(var_matches))

    # Extract metric names
    metric_matches = re.findall(r'\b(reth_\w+|process_\w+|go_\w+|node_\w+)', query)
    patterns['metrics'] = list(set(metric_matches))

    # Extract functions
    func_matches = re.findall(r'\b(rate|irate|increase|sum|avg|max|min|count|topk|bottomk)\s*\(', query)
    patterns['functions'] = list(set(func_matches))

    return patterns

def compare_queries_detailed(upstream_queries: List[Dict], scroll_queries: List[Dict]) -> List[Dict]:
    """Compare queries in detail"""
    differences = []

    # Create lookup by panel name
    upstream_by_panel = {q['panel']: q for q in upstream_queries}
    scroll_by_panel = {q['panel']: q for q in scroll_queries}

    for panel_name in set(upstream_by_panel.keys()) | set(scroll_by_panel.keys()):
        up_query = upstream_by_panel.get(panel_name)
        sc_query = scroll_by_panel.get(panel_name)

        if up_query and sc_query:
            if up_query['expr'] != sc_query['expr']:
                up_patterns = analyze_query_patterns(up_query['expr'])
                sc_patterns = analyze_query_patterns(sc_query['expr'])

                differences.append({
                    'panel': panel_name,
                    'status': 'modified',
                    'upstream_query': up_query['expr'],
                    'scroll_query': sc_query['expr'],
                    'upstream_patterns': up_patterns,
                    'scroll_patterns': sc_patterns,
                    'added_labels': [l for l in sc_patterns['label_filters'] if l not in up_patterns['label_filters']],
                    'removed_labels': [l for l in up_patterns['label_filters'] if l not in sc_patterns['label_filters']],
                    'added_variables': [v for v in sc_patterns['variables'] if v not in up_patterns['variables']],
                    'removed_variables': [v for v in up_patterns['variables'] if v not in sc_patterns['variables']],
                })
        elif up_query and not sc_query:
            differences.append({
                'panel': panel_name,
                'status': 'only_upstream',
                'upstream_query': up_query['expr'],
            })
        elif sc_query and not up_query:
            differences.append({
                'panel': panel_name,
                'status': 'only_scroll',
                'scroll_query': sc_query['expr'],
            })

    return differences

def analyze_variables_detailed(upstream_vars: List[Dict], scroll_vars: List[Dict]) -> Dict:
    """Analyze variable differences in detail"""
    upstream_by_name = {v['name']: v for v in upstream_vars}
    scroll_by_name = {v['name']: v for v in scroll_vars}

    analysis = {
        'added_in_scroll': [],
        'removed_in_scroll': [],
        'modified': [],
    }

    for name, var in scroll_by_name.items():
        if name not in upstream_by_name:
            analysis['added_in_scroll'].append(var)
        elif upstream_by_name[name] != var:
            analysis['modified'].append({
                'name': name,
                'upstream': upstream_by_name[name],
                'scroll': var,
            })

    for name, var in upstream_by_name.items():
        if name not in scroll_by_name:
            analysis['removed_in_scroll'].append(var)

    return analysis

def generate_update_plan(filename: str, upstream_path: Path, scroll_path: Path) -> str:
    """Generate detailed update plan for a dashboard"""
    upstream = load_json(upstream_path)
    scroll = load_json(scroll_path)

    up_queries = extract_all_queries(upstream)
    sc_queries = extract_all_queries(scroll)

    query_diffs = compare_queries_detailed(up_queries, sc_queries)

    up_vars = upstream.get('templating', {}).get('list', [])
    sc_vars = scroll.get('templating', {}).get('list', [])

    var_analysis = analyze_variables_detailed(up_vars, sc_vars)

    plan = []
    plan.append(f"## UPDATE PLAN: {filename}")
    plan.append("=" * 80)
    plan.append("")

    # Dashboard metadata
    plan.append(f"**Dashboard**: {upstream.get('title')}")
    plan.append(f"**Upstream UID**: {upstream.get('uid')}")
    plan.append(f"**Scroll UID**: {scroll.get('uid')}")
    plan.append("")

    # Variable analysis
    plan.append("### VARIABLES")
    plan.append("")

    if var_analysis['added_in_scroll']:
        plan.append("**Kubernetes Variables Added in Scroll** (MUST PRESERVE):")
        for var in var_analysis['added_in_scroll']:
            plan.append(f"  - `{var['name']}` (type: {var.get('type')})")
            if var.get('query'):
                plan.append(f"    Query: `{var['query']}`")
        plan.append("")

    if var_analysis['removed_in_scroll']:
        plan.append("**Variables Removed in Scroll** (SHOULD ADD BACK FROM UPSTREAM):")
        for var in var_analysis['removed_in_scroll']:
            plan.append(f"  - `{var['name']}` (type: {var.get('type')})")
            if var.get('query'):
                plan.append(f"    Query: `{var['query']}`")
        plan.append("")

    # Query analysis
    plan.append("### QUERY ANALYSIS")
    plan.append("")

    k8s_customizations = [d for d in query_diffs if d['status'] == 'modified' and d['added_labels']]
    upstream_only_panels = [d for d in query_diffs if d['status'] == 'only_upstream']
    scroll_only_panels = [d for d in query_diffs if d['status'] == 'only_scroll']

    if k8s_customizations:
        plan.append(f"**Kubernetes Customizations** ({len(k8s_customizations)} panels):")
        plan.append("")
        for diff in k8s_customizations[:5]:  # Show first 5 examples
            plan.append(f"**Panel: {diff['panel']}**")
            if diff['added_labels']:
                plan.append(f"  Added label filters: {diff['added_labels']}")
            if diff['added_variables']:
                plan.append(f"  Added variables: {diff['added_variables']}")
            plan.append("")
        if len(k8s_customizations) > 5:
            plan.append(f"  ... and {len(k8s_customizations) - 5} more panels with customizations")
            plan.append("")

    if upstream_only_panels:
        plan.append(f"**Panels Only in Upstream** ({len(upstream_only_panels)} panels) - NEW FEATURES:")
        for panel in upstream_only_panels:
            plan.append(f"  - {panel['panel']}")
        plan.append("")

    if scroll_only_panels:
        plan.append(f"**Panels Only in Scroll** ({len(scroll_only_panels)} panels) - CUSTOM ADDITIONS:")
        for panel in scroll_only_panels:
            plan.append(f"  - {panel['panel']}")
        plan.append("")

    # Update strategy
    plan.append("### UPDATE STRATEGY")
    plan.append("")
    plan.append("**Step 1: Variable Reconciliation**")

    if var_analysis['added_in_scroll']:
        plan.append("  - Keep Kubernetes variables from scroll version:")
        for var in var_analysis['added_in_scroll']:
            plan.append(f"    * `{var['name']}`")

    if var_analysis['removed_in_scroll']:
        plan.append("  - Add back missing upstream variables:")
        for var in var_analysis['removed_in_scroll']:
            plan.append(f"    * `{var['name']}`")
    plan.append("")

    plan.append("**Step 2: Panel Structure**")
    plan.append("  - Use upstream panel structure as base (including new panels)")
    if upstream_only_panels:
        plan.append(f"  - Add {len(upstream_only_panels)} new panels from upstream")
    if scroll_only_panels:
        plan.append(f"  - Evaluate {len(scroll_only_panels)} scroll-only panels for retention")
    plan.append("")

    plan.append("**Step 3: Query Migration**")
    plan.append("  - For each query, apply Kubernetes label filters:")

    # Detect common K8s pattern
    k8s_labels_to_add = set()
    for diff in k8s_customizations:
        for label, _ in diff['added_labels']:
            k8s_labels_to_add.add(label)

    if k8s_labels_to_add:
        plan.append(f"    * Add label filters: {', '.join(sorted(k8s_labels_to_add))}")

    # Detect variable pattern changes
    var_replacements = set()
    for diff in k8s_customizations:
        if diff['added_variables']:
            for v in diff['added_variables']:
                var_replacements.add(v)

    if var_replacements:
        plan.append(f"    * Add variable references: {', '.join(sorted([f'${v}' for v in var_replacements]))}")
    plan.append("")

    plan.append("**Step 4: Testing Requirements**")
    plan.append("  - Verify all queries work in Kubernetes environment")
    plan.append("  - Check variable dropdowns populate correctly")
    plan.append("  - Validate metric label selectors match Helm deployment")
    plan.append("")

    # Specific example
    if k8s_customizations:
        plan.append("### EXAMPLE TRANSFORMATION")
        plan.append("")
        example = k8s_customizations[0]
        plan.append(f"**Panel: {example['panel']}**")
        plan.append("")
        plan.append("**Upstream Query:**")
        plan.append("```promql")
        plan.append(example['upstream_query'][:500])
        plan.append("```")
        plan.append("")
        plan.append("**Scroll Query (with K8s filters):**")
        plan.append("```promql")
        plan.append(example['scroll_query'][:500])
        plan.append("```")
        plan.append("")
        plan.append("**Changes Applied:**")
        if example['added_labels']:
            plan.append(f"  - Added label filters: {example['added_labels']}")
        if example['added_variables']:
            plan.append(f"  - Added variables: {example['added_variables']}")
        plan.append("")

    plan.append("=" * 80)
    plan.append("")

    return "\n".join(plan)

def main():
    upstream_dir = Path('etc/grafana/dashboards')
    scroll_dir = Path('etc/grafana/scroll')

    # Find all dashboards
    upstream_files = {f.name: f for f in upstream_dir.glob('*.json')}
    scroll_files = {f.name: f for f in scroll_dir.glob('*.json')}

    common_files = set(upstream_files.keys()) & set(scroll_files.keys())

    print("=" * 80)
    print("DETAILED DASHBOARD ANALYSIS AND UPDATE PLANS")
    print("=" * 80)
    print()

    # Case A: Detailed analysis
    for filename in sorted(common_files):
        plan = generate_update_plan(filename, upstream_files[filename], scroll_files[filename])
        print(plan)

    # Case B: metrics-exporter.json
    print("\n" + "=" * 80)
    print("CASE B: UPSTREAM-ONLY DASHBOARDS")
    print("=" * 80)
    print()

    only_upstream = set(upstream_files.keys()) - set(scroll_files.keys())
    for filename in sorted(only_upstream):
        upstream = load_json(upstream_files[filename])
        print(f"## {filename}")
        print()
        print(f"**Title**: {upstream.get('title')}")
        print(f"**UID**: {upstream.get('uid')}")
        print(f"**Panels**: {len(upstream.get('panels', []))}")
        print(f"**Variables**: {len(upstream.get('templating', {}).get('list', []))}")
        print()
        print("**Recommendation**:")

        # Analyze if it's relevant
        queries = extract_all_queries(upstream)
        metrics = set()
        for q in queries:
            patterns = analyze_query_patterns(q['expr'])
            metrics.update(patterns['metrics'])

        print(f"  - Metrics used: {', '.join(sorted(metrics)[:5])}")
        print(f"  - Total unique metrics: {len(metrics)}")
        print()

        if 'ethereum' in upstream.get('title', '').lower():
            print("  **Action**: Consider porting to scroll/ with K8s adaptations")
            print("  **Reason**: Ethereum-specific metrics likely relevant for Scroll")
        else:
            print("  **Action**: Evaluate relevance for Scroll deployment")
        print()
        print("  **If porting:**")
        print("    1. Copy from upstream as base")
        print("    2. Add K8s variables (env, pod, service)")
        print("    3. Add label filters to all queries: pod=~\"$pod\", etc.")
        print("    4. Update datasource references if needed")
        print()

    # Case C: scroll-only
    print("\n" + "=" * 80)
    print("CASE C: SCROLL-ONLY DASHBOARDS")
    print("=" * 80)
    print()

    only_scroll = set(scroll_files.keys()) - set(upstream_files.keys())
    for filename in sorted(only_scroll):
        scroll = load_json(scroll_files[filename])
        print(f"## {filename}")
        print()
        print(f"**Title**: {scroll.get('title')}")
        print(f"**UID**: {scroll.get('uid')}")
        print(f"**Panels**: {len(scroll.get('panels', []))}")
        print(f"**Variables**: {len(scroll.get('templating', {}).get('list', []))}")
        print()

        queries = extract_all_queries(scroll)
        metrics = set()
        for q in queries:
            patterns = analyze_query_patterns(q['expr'])
            metrics.update(patterns['metrics'])

        print(f"  - Metrics used: {', '.join(sorted(metrics))}")
        print()
        print("**Recommendation**:")
        print("  - **Action**: Keep as scroll-specific custom dashboard")
        print("  - **Reason**: Custom performance monitoring for Scroll deployment")
        print("  - **Maintenance**: Update as needed for scroll-specific requirements")
        print()

if __name__ == '__main__':
    main()

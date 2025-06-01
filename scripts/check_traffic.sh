#!/bin/bash
# GitHub Traffic Monitor for Custom Compositor
# Usage: ./check_traffic.sh

echo "Custom Compositor GitHub Traffic Report"
echo "=========================================="
echo ""

echo "Repository Views (Last 14 days):"
gh api repos/:owner/:repo/traffic/views | jq '.count as $total | .uniques as $unique | "Total Views: \($total) | Unique Visitors: \($unique)"' -r
echo ""

echo "Repository Clones (Last 14 days):"
gh api repos/:owner/:repo/traffic/clones | jq '.count as $total | .uniques as $unique | "Total Clones: \($total) | Unique Cloners: \($unique)"' -r
echo ""

echo "Top Referral Sources:"
gh api repos/:owner/:repo/traffic/popular/referrers | jq '.[] | "  \(.referrer): \(.count) views (\(.uniques) unique)"' -r
echo ""

echo "Repository Stats:"
gh repo view --json stargazerCount,forkCount,watchers | jq '"Stars: \(.stargazerCount) | Forks: \(.forkCount) | Watchers: \(.watchers.totalCount)"' -r
echo ""

echo "Recent Daily Views:"
gh api repos/:owner/:repo/traffic/views | jq '.views[] | "  \(.timestamp[:10]): \(.count) views (\(.uniques) unique)"' -r

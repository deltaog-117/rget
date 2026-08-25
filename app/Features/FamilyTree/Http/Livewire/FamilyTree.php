<?php

declare(strict_types=1);

namespace App\Features\FamilyTree\Http\Livewire;

use App\Features\DistroComparison\Models\Distribution;
use Livewire\Component;
use Illuminate\Support\Collection;

class FamilyTree extends Component
{
    public function render()
    {
        // Fetch all distributions
        $distros = Distribution::all();

        // Build a tree: parent → children
        $tree = $this->buildTree($distros);

        return view('features.family-tree.tree', [
            'tree' => $tree,
        ]);
    }

    /**
     * Build a hierarchical tree from the list of distributions.
     * "Independent" or null based_on are root nodes.
     */
    private function buildTree(Collection $distros): array
    {
        // Map slug → node data
        $nodes = [];
        $roots = [];

        // First pass: create node objects
        foreach ($distros as $distro) {
            $nodes[$distro->slug] = [
                'name' => $distro->name,
                'slug' => $distro->slug,
                'based_on' => $distro->based_on,
                'children' => [],
            ];
        }

        // Second pass: assign children
        foreach ($nodes as $slug => &$node) {
            $parentSlug = $node['based_on'] ? strtolower(str_replace(' ', '-', $node['based_on'])) : null;

            // Try to find parent by slug (we stored slugs in the table)
            // If not found, treat as root
            if ($parentSlug && isset($nodes[$parentSlug])) {
                $nodes[$parentSlug]['children'][] = &$node;
            } else {
                // Also check if parentSlug matches any distribution name (case-insensitive)
                $found = false;
                foreach ($nodes as $possibleParent) {
                    if (strtolower($possibleParent['name']) === strtolower($node['based_on'] ?? '')) {
                        $possibleParent['children'][] = &$node;
                        $found = true;
                        break;
                    }
                }
                if (!$found) {
                    $roots[] = &$node;
                }
            }
        }

        // Clean up: remove nodes that have no children and are not roots? Keep all.
        // But we only need the root nodes for display.
        return $roots;
    }
}

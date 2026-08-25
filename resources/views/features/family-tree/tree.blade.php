<div>
    <div id="tree-container" style="width: 100%; height: 700px; overflow: auto; border: 1px solid #e2e8f0; border-radius: 0.5rem; background: #f9fafb;">
        <!-- D3 tree will be rendered here -->
    </div>

    <!-- Include D3.js from CDN -->
    <script src="https://d3js.org/d3.v7.min.js"></script>

    <script>
        document.addEventListener('livewire:init', function () {
            // Wait for Livewire to render the component, then draw the tree
            Livewire.on('rendered', function () {
                drawTree();
            });
        });

        // Initial draw after page load
        document.addEventListener('DOMContentLoaded', function () {
            setTimeout(drawTree, 200);
        });

        function drawTree() {
            // Get the tree data from the blade variable
            const treeData = @json($tree);

            if (!treeData || treeData.length === 0) {
                document.getElementById('tree-container').innerHTML = '<p class="text-center text-gray-500 p-4">No distribution data available.</p>';
                return;
            }

            // Convert flat array of roots into a D3 hierarchy
            // We need to build a single root, but we have multiple roots (independent distributions).
            // We can create a virtual root node.
            const rootNode = { name: 'Linux', children: treeData };

            // Compute the tree layout
            const margin = { top: 20, right: 120, bottom: 20, left: 120 };
            const width = document.getElementById('tree-container').clientWidth - margin.left - margin.right;
            const height = document.getElementById('tree-container').clientHeight - margin.top - margin.bottom;

            // Clear previous SVG
            const container = document.getElementById('tree-container');
            container.innerHTML = '';

            // Create SVG
            const svg = d3.select(container)
                .append('svg')
                .attr('width', width + margin.left + margin.right)
                .attr('height', height + margin.top + margin.bottom)
                .append('g')
                .attr('transform', 'translate(' + margin.left + ',' + margin.top + ')');

            const root = d3.hierarchy(rootNode, function(d) {
                return d.children;
            });

            // Compute tree layout
            const treeLayout = d3.tree()
                .size([height, width]);

            const treeData2 = treeLayout(root);

            // Draw links
            svg.selectAll('.link')
                .data(treeData2.links())
                .enter()
                .append('path')
                .attr('class', 'link')
                .attr('fill', 'none')
                .attr('stroke', '#888')
                .attr('stroke-width', 2)
                .attr('d', d3.linkHorizontal()
                    .x(function(d) { return d.y; })
                    .y(function(d) { return d.x; })
                );

            // Draw nodes
            const node = svg.selectAll('.node')
                .data(treeData2.descendants())
                .enter()
                .append('g')
                .attr('class', 'node')
                .attr('transform', function(d) {
                    return 'translate(' + d.y + ',' + d.x + ')';
                });

            // Circles for nodes
            node.append('circle')
                .attr('r', 6)
                .attr('fill', function(d) {
                    return d.depth === 0 ? '#2b6cb0' : '#4299e1';
                })
                .attr('stroke', '#fff')
                .attr('stroke-width', 2);

            // Labels
            node.append('text')
                .attr('dy', '.35em')
                .attr('x', function(d) {
                    return d.children ? -10 : 10;
                })
                .attr('text-anchor', function(d) {
                    return d.children ? 'end' : 'start';
                })
                .style('font-size', '12px')
                .style('font-family', 'sans-serif')
                .text(function(d) {
                    return d.data.name;
                });
        }
    </script>
</div>

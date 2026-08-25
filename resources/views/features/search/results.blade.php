<div>
    @if(!empty($query))
        @if($results && $results->count() > 0)
            <p class="text-gray-600 mb-4">Found {{ $results->total() }} result(s) for "{{ $query }}"</p>
            <div class="space-y-4">
                @foreach($results as $page)
                    <div class="bg-white p-4 border rounded shadow-sm hover:shadow transition">
                        <h2 class="text-xl font-semibold text-blue-600">
                            <a href="{{ route('wiki.show', $page->slug) }}">{{ $page->title }}</a>
                        </h2>
                        <p class="text-gray-600 text-sm mt-1">
                            {{ Str::limit(strip_tags($page->content), 200) }}
                        </p>
                        <div class="text-xs text-gray-400 mt-2">
                            Updated {{ $page->updated_at->diffForHumans() }}
                            @if(isset($page->relevance))
                                <span class="ml-2 bg-gray-200 px-2 py-0.5 rounded">Relevance: {{ number_format($page->relevance, 2) }}</span>
                            @endif
                        </div>
                    </div>
                @endforeach
            </div>
            <div class="mt-6">
                {{ $results->appends(['q' => request('q')])->links() }}
            </div>
        @else
            <p class="text-gray-600">No results found for "{{ $query }}".</p>
        @endif
    @else
        <p class="text-gray-600">Enter a search term above.</p>
    @endif
</div>

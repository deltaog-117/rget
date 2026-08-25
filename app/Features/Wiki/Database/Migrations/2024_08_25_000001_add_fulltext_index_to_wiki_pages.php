<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;
use Illuminate\Support\Facades\DB;

return new class extends Migration
{
    public function up(): void
    {
        // Add FULLTEXT index for title and content
        DB::statement('ALTER TABLE wiki_pages ADD FULLTEXT fulltext_index (title, content)');
    }

    public function down(): void
    {
        DB::statement('ALTER TABLE wiki_pages DROP INDEX fulltext_index');
    }
};

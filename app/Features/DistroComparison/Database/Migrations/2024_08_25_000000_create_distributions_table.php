<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('distributions', function (Blueprint $table) {
            $table->id();
            $table->string('name');
            $table->string('slug')->unique();
            $table->string('based_on')->nullable();
            $table->string('package_manager')->nullable();
            $table->string('default_desktop')->nullable();
            $table->string('release_model')->nullable(); // e.g., "Rolling", "Fixed"
            $table->string('architecture')->nullable(); // e.g., "x86_64", "ARM"
            $table->text('description')->nullable();
            $table->string('website')->nullable();
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('distributions');
    }
};

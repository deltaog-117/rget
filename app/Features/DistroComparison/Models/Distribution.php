<?php

declare(strict_types=1);

namespace App\Features\DistroComparison\Models;

use Illuminate\Database\Eloquent\Model;

class Distribution extends Model
{
    protected $table = 'distributions';

    protected $fillable = [
        'name',
        'slug',
        'based_on',
        'package_manager',
        'default_desktop',
        'release_model',
        'architecture',
        'description',
        'website',
    ];

    protected $casts = [
        'created_at' => 'datetime',
        'updated_at' => 'datetime',
    ];
}

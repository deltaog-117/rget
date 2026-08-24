<?php

declare(strict_types=1);

namespace App\Features\Wiki\ValueObjects;

use InvalidArgumentException;

final class Title
{
    private string $value;

    private function __construct(string $value)
    {
        $this->value = $value;
    }

    public static function fromString(string $value): self
    {
        $value = trim($value);
        if ($value === '') {
            throw new InvalidArgumentException('Title cannot be empty.');
        }
        if (strlen($value) > 255) {
            throw new InvalidArgumentException('Title cannot exceed 255 characters.');
        }
        return new self($value);
    }

    public function toString(): string
    {
        return $this->value;
    }

    public function __toString(): string
    {
        return $this->value;
    }
}

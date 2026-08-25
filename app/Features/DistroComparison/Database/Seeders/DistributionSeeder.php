<?php

namespace App\Features\DistroComparison\Database\Seeders;

use Illuminate\Database\Seeder;
use App\Features\DistroComparison\Models\Distribution;

class DistributionSeeder extends Seeder
{
    public function run(): void
    {
        $distros = [
            [
                'name' => 'Debian',
                'slug' => 'debian',
                'based_on' => 'Independent',
                'package_manager' => 'APT (dpkg)',
                'default_desktop' => 'GNOME (optional)',
                'release_model' => 'Fixed (Stable/Testing/Unstable)',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'One of the oldest and most stable Linux distributions, known for its commitment to free software.',
                'website' => 'https://www.debian.org',
            ],
            [
                'name' => 'Ubuntu',
                'slug' => 'ubuntu',
                'based_on' => 'Debian',
                'package_manager' => 'APT (dpkg)',
                'default_desktop' => 'GNOME',
                'release_model' => 'Fixed (LTS and interim)',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'The most popular Linux distribution for desktop and cloud, backed by Canonical.',
                'website' => 'https://ubuntu.com',
            ],
            [
                'name' => 'Arch Linux',
                'slug' => 'arch',
                'based_on' => 'Independent',
                'package_manager' => 'Pacman',
                'default_desktop' => 'None (user chooses)',
                'release_model' => 'Rolling',
                'architecture' => 'x86_64, ARM (Arch Linux ARM)',
                'description' => 'A lightweight and flexible rolling‑release distribution aimed at advanced users.',
                'website' => 'https://archlinux.org',
            ],
            [
                'name' => 'Fedora',
                'slug' => 'fedora',
                'based_on' => 'Red Hat',
                'package_manager' => 'DNF (RPM)',
                'default_desktop' => 'GNOME',
                'release_model' => 'Fixed (semiannual)',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'Cutting‑edge distribution sponsored by Red Hat, often used as a testbed for new technologies.',
                'website' => 'https://getfedora.org',
            ],
            [
                'name' => 'openSUSE',
                'slug' => 'opensuse',
                'based_on' => 'Independent (SUSE)',
                'package_manager' => 'ZYpp (RPM)',
                'default_desktop' => 'GNOME (optional)',
                'release_model' => 'Fixed (Leap) and Rolling (Tumbleweed)',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'A distribution offering both a stable (Leap) and rolling (Tumbleweed) release, with strong enterprise roots.',
                'website' => 'https://www.opensuse.org',
            ],
            [
                'name' => 'Linux Mint',
                'slug' => 'linux-mint',
                'based_on' => 'Ubuntu (LTS)',
                'package_manager' => 'APT (dpkg)',
                'default_desktop' => 'Cinnamon',
                'release_model' => 'Fixed (LTS)',
                'architecture' => 'x86_64',
                'description' => 'User‑friendly distribution focused on elegance and ease of use, popular among beginners.',
                'website' => 'https://linuxmint.com',
            ],
            [
                'name' => 'Gentoo',
                'slug' => 'gentoo',
                'based_on' => 'Independent',
                'package_manager' => 'Portage (emerge)',
                'default_desktop' => 'None (user builds)',
                'release_model' => 'Rolling (source‑based)',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'Source‑based distribution that offers extreme customizability and performance, but requires manual compilation.',
                'website' => 'https://www.gentoo.org',
            ],
            [
                'name' => 'Alpine Linux',
                'slug' => 'alpine',
                'based_on' => 'Independent',
                'package_manager' => 'APK',
                'default_desktop' => 'None (minimal)',
                'release_model' => 'Rolling',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'Security‑focused, musl‑based lightweight distribution, often used for containers and embedded systems.',
                'website' => 'https://alpinelinux.org',
            ],
            [
                'name' => 'NixOS',
                'slug' => 'nixos',
                'based_on' => 'Independent',
                'package_manager' => 'Nix',
                'default_desktop' => 'None (declarative)',
                'release_model' => 'Rolling and stable channels',
                'architecture' => 'x86_64, ARM, more',
                'description' => 'Unique distribution built on the Nix package manager; system configuration is declarative and reproducible.',
                'website' => 'https://nixos.org',
            ],
        ];

        foreach ($distros as $data) {
            Distribution::firstOrCreate(['slug' => $data['slug']], $data);
        }
    }
}

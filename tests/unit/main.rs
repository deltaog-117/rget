// SPDX-License-Identifier: GPL-3.0-or-later
// rget - A safe, modern downloader for Linux
// Copyright (C) 2026  Aeon Ennoia
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.


//! Unit tests for the public API, mirroring `src/`.
//!
//! Many of these are characterization tests: they pin what 1.0.0 does today so the
//! restructure cannot change it silently. Tests that record a known defect say so, and
//! flip when the roadmap item that fixes it lands.

mod destination;
mod input;
mod integrity;
mod shared;
mod validation;

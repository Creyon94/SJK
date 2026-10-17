// The holocron's loot-box tiers (SJK; see README.md): each tier's face (the Jedi
// cube's plates, the Great Holocron's pentagons, the Sith pyramid's sides, the kyber
// crystal). Lit, also glowing faintly of itself, and its lit parts kept bright and in
// the dynamic glow, breathing; the tiers climb in how bright. Mythical also breathes
// the whole face and has a sheen stripe sliding across.
// Made by scripts/holocron_assets.py from its TIERS table: edit that, not this.
models/sjk/holocron_uncommon
{
	q3map_nolightmap
	{
		map models/sjk/holocron_uncommon
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_uncommon
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.35 0 0 0
	}
	{
		map models/sjk/holocron_uncommon_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.4 0.1 0 0.3
		glow
	}
}
models/sjk/holocron_rare
{
	q3map_nolightmap
	{
		map models/sjk/holocron_rare
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_rare
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.4 0 0 0
	}
	{
		map models/sjk/holocron_rare_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.5 0.15 0 0.35
		glow
	}
}
models/sjk/holocron_legendary
{
	q3map_nolightmap
	{
		map models/sjk/holocron_legendary
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_legendary
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.45 0 0 0
	}
	{
		map models/sjk/holocron_legendary_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.6 0.2 0 0.4
		glow
	}
}
models/sjk/holocron_mythical
{
	q3map_nolightmap
	{
		map models/sjk/holocron_mythical
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_mythical
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.45 0.2 0.25 0.35
	}
	{
		map models/sjk/holocron_mythical_glow
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.85 0.35 0 0.55
		glow
	}
	{
		map models/sjk/holocron_mythical_sheen
		blendFunc GL_ONE GL_ONE
		tcMod scale 0.5 0.5
		tcMod scroll 0.06 0.06
		rgbGen wave sin 0.5 0.15 0 0.25
	}
}

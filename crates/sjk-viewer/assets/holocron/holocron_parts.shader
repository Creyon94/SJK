// The holocrons' other materials (SJK; see README.md): steel (the cube's edges, the
// pyramid's foot, the kyber crystals' sheaths and rings), the Sith pyramid's dark
// metal, each tier's light (lamps, edges, halo, seam, eye, motes, sparks): full
// bright, breathing in the glow as the tier's face does; and the engram's seams
// (pulsing, breathing with the plates) and its white-hot core (pulsing in the glow).
// Made by scripts/holocron_assets.py: edit that, not this.
models/sjk/holocron_steel
{
	q3map_nolightmap
	{
		map models/sjk/holocron_steel
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_steel
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.3 0 0 0
	}
}
models/sjk/holocron_dark
{
	q3map_nolightmap
	{
		map models/sjk/holocron_dark
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/holocron_dark
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.35 0 0 0
	}
}
models/sjk/holocron_uncommon_light
{
	q3map_nolightmap
	{
		map models/sjk/holocron_uncommon_light
		rgbGen identity
	}
	{
		map models/sjk/holocron_uncommon_light
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.4 0.1 0 0.3
		glow
	}
}
models/sjk/holocron_rare_light
{
	q3map_nolightmap
	{
		map models/sjk/holocron_rare_light
		rgbGen identity
	}
	{
		map models/sjk/holocron_rare_light
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.5 0.15 0 0.35
		glow
	}
}
models/sjk/holocron_legendary_light
{
	q3map_nolightmap
	{
		map models/sjk/holocron_legendary_light
		rgbGen identity
	}
	{
		map models/sjk/holocron_legendary_light
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.6 0.2 0 0.4
		glow
	}
}
models/sjk/holocron_mythical_light
{
	q3map_nolightmap
	{
		map models/sjk/holocron_mythical_light
		rgbGen identity
	}
	{
		map models/sjk/holocron_mythical_light
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.85 0.35 0 0.55
		glow
	}
}
models/sjk/holocron_mythical_seam
{
	q3map_nolightmap
	deformVertexes wave 1000 sin 0.2 0.2 0 0.35
	{
		map models/sjk/holocron_mythical_seam
		rgbGen identity
	}
	{
		map models/sjk/holocron_mythical_seam
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.3 0.25 0 0.8
		glow
	}
}
models/sjk/holocron_mythical_core
{
	q3map_nolightmap
	{
		map models/sjk/holocron_mythical_core
		rgbGen identity
	}
	{
		map models/sjk/holocron_mythical_core
		blendFunc GL_ONE GL_ONE
		rgbGen wave sin 0.6 0.4 0.5 0.8
		glow
	}
}

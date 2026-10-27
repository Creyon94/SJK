// The mystery box's chest and lid (SJK; see README.md): wood and iron lit by the
// world's light as a player model is, the question marks' glow added on top, pulsing.
models/sjk/mystery_box
{
	{
		map models/sjk/mystery_box
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/mystery_box_glow
		blendFunc add
		rgbGen wave sin 0.75 0.25 0 0.5
	}
}

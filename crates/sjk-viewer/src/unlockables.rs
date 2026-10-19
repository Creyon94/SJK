//! The unlockables this client knows (`docs/unlockables.md`, "Catalogue"): the one
//! catalogue the looks, the blade skins, the `saberskin` command, the Unlockables page
//! and the Staff page read. The hub keeps its own copy (`src/unlocks.rs`); a new
//! unlockable needs both, and an id never changes meaning. An id this client does not
//! know is ignored wherever it comes from (a profile, a look, `cg_saberSkin`).
//!
//! The catalogue is public metadata only: what an unlockable looks and sounds like is
//! art in the SJK hub's packs (`sjk_packs.rs`), a blade skin's in its blade-skin file.

/// Archived: the blade-skin unlock id the player wears, empty for the stock blade. It
/// shows (and is sent to the hub) only while the player's own hub profile lists it.
pub(crate) const SABER_SKIN_CVAR: &str = "cg_saberSkin";

/// What an unlockable is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    /// A look a player's blades wear instead of their colour, drawn from its blade-skin
    /// file (`skins/blades/<id>.bladeskin` in a pack).
    BladeSkin,
}

/// How rare an unlockable is, the commonest first: Common (14/10/2026, white: filler
/// that holocrons of any tier may drop; there is no Common holocron) and the holocrons'
/// four tiers ([`crate::holocrons::TIERS`]) with their names and colours.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
    Mythical,
}

impl Rarity {
    /// The holocron tier of the same name; `None` for Common, which no holocron is.
    fn holocron_tier(self) -> Option<&'static crate::holocrons::Tier> {
        (self as usize)
            .checked_sub(1)
            .map(|index| &crate::holocrons::TIERS[index])
    }

    /// The tier's name ("Rare").
    pub(crate) fn label(self) -> &'static str {
        self.holocron_tier().map_or("Common", |tier| tier.label)
    }

    /// The tier's colour: Common's is white (a touch cool, to sit on the menus).
    pub(crate) fn colour(self) -> sjk_ui::Color {
        self.holocron_tier()
            .map_or(crate::holocrons::rgb(0xE6_EAF2), |tier| tier.colour)
    }

    /// The tier's mark in SJK Symbols, Common's the empty octagon.
    pub(crate) fn symbol(self) -> char {
        crate::symbols::RARITY[self as usize]
    }

    /// Its place among the effects' ranks ([`crate::rarity_fx`]): Common and Uncommon
    /// are 0 and draw none.
    pub(crate) fn effect_rank(self) -> usize {
        (self as usize).saturating_sub(1)
    }
}

/// The saber smiths: who gave a shader's idea ([`Unlockable::smith`]).
pub(crate) const SOL: &str = "Sol";
pub(crate) const CREYON: &str = "Creyon";

/// How to get a shader that holocrons drop ([`Unlockable::how_to_get`]).
pub(crate) const FROM_HOLOCRONS: &str = "Found in holocrons, or given by the SJK team.";
/// How to get a shader the holocrons do not drop yet: Creyon's Blade Lab signatures.
pub(crate) const FROM_THE_TEAM: &str =
    "Not in holocrons yet: given by the SJK team while it is tried.";

/// One unlockable of the catalogue.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Unlockable {
    /// The id the hub, a look and `cg_saberSkin` name it by.
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    /// How rare it is: the holocrons' four tiers, their names and colours.
    pub(crate) tier: Rarity,
    /// A chroma: a blade skin that takes the wearer's saber colour (`color1` for the
    /// first saber, `color2` for the second), its colours turned round to that hue
    /// ([`crate::saber_skins::Chroma`]). The others keep the colours their file gives.
    pub(crate) chroma: bool,
    /// Whether a holocron may drop it (the hub's `droppable`, `docs/unlockables.md`,
    /// "Holocron shader drops"): since 17/10/2026 every shader but those a medal brings
    /// ([`crate::medals::Medal::blade`]).
    pub(crate) droppable: bool,
    /// The edition it belongs to ([`crate::editions`]): the stamp of when it came.
    pub(crate) edition: &'static str,
    /// Its saber smith: who gave the idea, credited on its cards ("Saber smith: Sol"),
    /// even when it was made together.
    pub(crate) smith: &'static str,
    /// What it is, in a line.
    pub(crate) description: &'static str,
    /// How a player gets it, in a line.
    pub(crate) how_to_get: &'static str,
}

impl Unlockable {
    /// Whether it is a blade skin.
    pub(crate) const fn is_blade_skin(&self) -> bool {
        matches!(self.kind, Kind::BladeSkin)
    }
}

/// Every unlockable, in the hub's catalogue order (the order profiles list them in).
pub(crate) const ALL: [Unlockable; 74] = [
    Unlockable {
        id: "saber_sun",
        name: "Sun blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A white-gold core in an orange corona: flares, prominences arching off it, sparks, a glint at its tip and heat haze.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_storm",
        name: "Storm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A white-blue core crackling with lightning that leaps off it, sparks at its tip.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_void",
        name: "Void blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A dark core in a violet rim, star specks drifting into it, and a deep hum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_frost",
        name: "Frost blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A pale ice core with a pointed tip, frost shards trailing it, and a chiming hum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_prism",
        name: "Prism blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A white core in a corona whose colours run through the rainbow, slowly turning.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_unstable",
        name: "Unstable blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A raw red core that crackles and sputters, its edge ragged and its length faltering.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_molten",
        name: "Molten blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A dark rock core split by glowing lava cracks, embers dripping off it and falling.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_spectral",
        name: "Spectral blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A pale, see-through blade: wisps rise off it, souls drift round it, an echo sways beside it and afterimages trail its swings.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_glitch",
        name: "Glitch blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A blade whose colours split apart, blocks of it jumping sideways now and then.",
        how_to_get: "Comes with the Bug Hunter medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_hologram",
        name: "Hologram blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A wireframe projection with scan lines running along it, jittering like a training hologram.",
        how_to_get: "Comes with the Early Tester medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_runic",
        name: "Runic blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A gold core with glyphs scrolling along it that spell its wearer's name.",
        how_to_get: "Comes with the Early Contributor medal, for as long as you hold it.",
    },
    Unlockable {
        id: "saber_chameleon",
        name: "Chameleon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A blade that takes the colour of the light where it is, changing from map to map.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_banner",
        name: "Banner blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "Your team's colour in team games, rippling like a flag; silver outside them.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_heartbeat",
        name: "Heartbeat blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: SOL,
        description: "A warm, deep blade that pulses with a slow double heartbeat, its light beating with it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_tidal",
        name: "Kamino blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A sea-blue blade from Kamino's endless ocean, a wave of brighter light surging from hilt to tip on a slow loop.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_circuit",
        name: "Droid Logic blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A thin cyan blade with sharp pulses of droid data racing along it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_aurora",
        name: "Hoth Aurora blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Slow ribbons of green, teal and violet drifting along the blade, like the sky over Hoth.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_echo",
        name: "Echo Base blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A sky-blue blade with a bright ring rippling along it and fading away, like a signal from Echo Base.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_nebula",
        name: "Nebula blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Deep indigo with tiny stars twinkling inside, a slice of the galaxy.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_cinder",
        name: "Mustafar Ember blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A dull coal-orange blade from Mustafar that flickers and sheds glowing embers.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_venom",
        name: "Geonosis Acid blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Acid green with droplets that bead up and drip off the blade, like the acid pits of Geonosis.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sakura",
        name: "Naboo Blossom blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A soft pink blade shedding blossom petals that drift away, like the gardens of Naboo.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_firefly",
        name: "Endor Lights blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A soft warm-green blade with the glowing lights of Endor's forest moon blinking around it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_slate",
        name: "Durasteel blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A cool steel-grey blade with a faint, slow shimmer, plain and sturdy as durasteel.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_amber",
        name: "Tatooine Gold blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A warm honey-gold blade with a soft glow that swells and settles, like Tatooine's sand at noon.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_lilac",
        name: "Lothal Twilight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A gentle lilac blade whose colour drifts a little as it moves, like the grass plains of Lothal at dusk.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_mint",
        name: "Yavin Jungle blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale, fresh green blade with a cool wave of light running along it, like the jungle moon of Yavin.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_coral",
        name: "Mon Cala Coral blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A salmon-pink blade with a warm pulse travelling from hilt to tip, like the coral cities of Mon Cala.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ivory",
        name: "Jedi Temple blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Common,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A creamy, near-white blade with a soft golden fringe, like the halls of the Jedi Temple.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_glacier",
        name: "Ilum blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Ice-blue with tiny crystals of frost glittering inside, from the frozen kyber world of Ilum.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_dusk",
        name: "Binary Sunset blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Orange sinking into violet, the colour of Tatooine's twin suns setting, slowly turning.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_ruby",
        name: "Sith Ruby blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A deep red blade shedding bright sparks as it burns.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_amethyst",
        name: "Kyber Amethyst blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Rich purple with tiny flecks of light twinkling in the kyber crystal.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_jade",
        name: "Dagobah Jade blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A calm sea-green blade whose hue slowly shifts, with specks of light inside, like the swamps of Dagobah.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_moonlight",
        name: "Ahch-To Moonlight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Silver-blue with pale motes drifting along it like dust in the moonlight over Ahch-To.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_neon",
        name: "Coruscant Neon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A glass neon tube from the lower levels of Coruscant, gas glowing inside, one of its six sections stuttering out now and then.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_peppermint",
        name: "Kessel Spice blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Uncommon,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale gold blade glittering with refined spice from Kessel: crystals sparkling inside it and glittering dust drifting off it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bubble",
        name: "Bacta blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade of glowing bacta: blue-green healing fluid with currents rising in it and bubbles of every size streaming up round it.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_lavalamp",
        name: "Coaxium blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Raw coaxium hyperfuel: glowing violet-red globules churning inside an unstable blade that spits sparks and short arcs.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_reactor",
        name: "Reactor blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A hypermatter reactor core: containment rings marching up the blade, magnetic loops arching between them, reactor dust; it heats to critical red, vents and flashes, then cools.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_bleeding",
        name: "Bleeding Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pure white kyber blade fighting a red corruption that bleeds up from the hilt, veins of it cracking into the air and red drops falling, while beams of light push it back.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_sketch",
        name: "Death Star Plans blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Rare,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The stolen battle station plans: a blueprint-blue blade with a scan line racing up it inside a turning holographic schematic.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_hourglass",
        name: "Beskar Forge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Mandalorian steel fresh from the forge: a silver blade with forge heat glowing up it, the hammer striking sparks and molten beskar dripping off.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_fiber",
        name: "Kyber Cluster blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A cluster of raw kyber crystals: a clear blade with a ring of glowing kyber shards floating round it, each linked to it by a beam of light.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_serpent",
        name: "Sith Alchemy blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The dark science of the Sith: an emerald blade veined with gold, green and gold lightning wound round it, green fire beams and alchemical vapour.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_monarch",
        name: "Felucia Spores blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The fungal jungle of Felucia: glowing spores rest on an amber-green blade while it is still and fly off round it in spirals when it swings, then settle back.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_phoenix",
        name: "Phoenix Squadron blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Rebellion's Phoenix: a blade of living flame wrapped in spirals of fire, beams of flame and sparks streaming up, reborn in a ring of fire every few seconds.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_quantum",
        name: "Mortis blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The realm of Mortis: a blade that turns from day to night, the Daughter's and the Son's ghost blades turning round it, rising light by day and red lightning by night.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_comic",
        name: "Clone Wars blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: true,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade drawn like the animated Clone Wars: flat colour, a thick black outline and its glow printed in halftone dots. It takes your saber colour.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_quicksilver",
        name: "Liquid Beskar blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Legendary,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A blade of molten beskar: ripples run up its mirror steel, beads roll to the tip, droplets orbit and fly off it, forge sparks fly and the air shimmers with heat.",
        how_to_get: FROM_HOLOCRONS,
    },
    Unlockable {
        id: "saber_obsidian",
        name: "Obsidian Edge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: true,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A flat blade of black volcanic glass with white-hot edges, obsidian shards orbiting it, magma lightning leaping between them and the air shimmering with heat.",
        how_to_get: FROM_HOLOCRONS,
    },
    // Creyon's Blade Lab signature shaders (blade_signatures.rs), mythicals first: not in
    // the holocrons yet, given by the team while they are tried in game.
    Unlockable {
        id: "saber_abyssal_eye",
        name: "Abyssal Eye blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A slit cut into an endless dark, its lips glowing crimson; it peels open into a great amber eye that rolls to watch along the blade and flinches shut on a clash.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_aurora_wyrm",
        name: "Ilum Aurora Storm blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Ilum's polar sky in a storm: green and magenta aurora curtains hanging round a pale-green blade, pale lightning crackling all round it, ice crystals glittering in the cold.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_balance",
        name: "Balance blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The light and the dark side in one blade: waves of blue-white and crimson rolling against each other, beams of pure light on one side and red lightning on the other, turning round it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_between_worlds",
        name: "Between Worlds blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A white-gold path of light from the World Between Worlds: pathways of light crossing the void round it, portals opening, and two lights, white and red, duelling round the blade.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_big_bang",
        name: "Big Bang blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A young universe: every ignition bursts from one blinding point into a fireball that lights the blade, then nebula clouds swirl round it, stars drift outward and small bangs throw shells of light off it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_carbonite",
        name: "Carbonite blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "Frozen in carbonite: a dark bronze slab holds the blade while cracks of light spread, bursts into shards that float round it while the blade blazes, then fuses solid again.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_dyson_sphere",
        name: "Dyson Sphere blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A star caged in a megastructure: five tilted hoops of light turning round a boiling white-gold blade, collector beams drinking from it, flame tongues and solar flares leaping up to the hoops.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_eclipse",
        name: "Eclipse blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pearl blade with a total eclipse travelling along its upper half: the moon's band darkens it, a corona of streamers and beams fans out, eclipsed suns circle it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_event_horizon",
        name: "Event Horizon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A black hole stretched into a blade: a rod of pure darkness with a razor photon ring, two accretion disks of glowing matter orbiting it, streams spiralling in and a gravity wave on a clash.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_exogorth",
        name: "Exogorth blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The space slug's living gullet: a ribbed, glowing throat with a maw of pale fangs at the emitter, swallowing waves, asteroid rocks and grit sucked in and crushed in sparks, hot breath and a roar on a clash.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_hyperspace",
        name: "Hyperspace blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A ship's jump as a blade: the stars stretched into streaks rushing up it inside a swirling blue tunnel; it jumps when lit and drops out of hyperspace on a clash.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_kyber_heart",
        name: "Kyber Heart blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A rose kyber blade that beats like a heart: every beat throws shock rings and beams of light off it, veins of light winding round it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_nebula_song",
        name: "Nebula Song blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A deep-space nebula that sings: a deep-blue blade breathing with slow waves of light, rings of song spreading off it, ribbons of glowing nebula gas winding round it and a hyperspace shimmer now and then.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_quasar",
        name: "Quasar blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A galaxy's engine: a white-violet jet with bright knots racing up it, a tilted accretion disk spiralling round it and matter flung off on every swing.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_singing_kyber",
        name: "Singing Kyber blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber crystal that sings: a clear faceted crystal over a teal blade, its standing wave's nodes blazing and trembling, sound rings rippling off it and a sonic boom on a clash.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_sith_holocron",
        name: "Sith Holocron blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A holocron's crimson crystal stretched into a blade: red lightning crackling all round it, a red heart beating inside and Sith knowledge streaming up the glass.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_star_dragon",
        name: "Star Dragon blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The power of the star dragons as a blade: a gold hide of shingled scales rippling up it, a slow inner fire, dragon breath of stardust-fire rolling up it full of star sparks; no dragon is drawn.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_starforge",
        name: "Starforge blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A kyber blade forged inside a star: solar beams and flares off a boiling white-gold blade, hammer blows of sparks, embers falling and the air shimmering with its heat.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_superlaser",
        name: "Superlaser blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A battle station's main weapon as a blade: eight beams fan in to a blazing, crackling focus above the hilt, plasma strands twist up the beam and surges race up it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_uneti_tree",
        name: "Uneti Tree blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Force-sensitive tree of Ahch-To as living wood-light: glowing fibres braided round a bright heartwood, sap of the Force pulsing up them, Force pollen and sparks of sap drifting off it.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_witchfire",
        name: "Witchfire blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The Nightsister magick of Dathomir: a sickly green ichor blade with pulses of magick climbing it, green ghost-fire licking off it, energy tendrils coiling round it, ichor mist and dark smoke.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_world_shift",
        name: "World Shift blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: false,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "The galaxy's worlds in turn: Mustafar fire, Hoth ice, a Kamino storm, Tatooine sand, Crait salt and Mon Cala water, each change an explosion that blasts the old climate off and turns the blade into the next.",
        how_to_get: FROM_THE_TEAM,
    },
    Unlockable {
        id: "saber_wraithlight",
        name: "Wraithlight blade",
        kind: Kind::BladeSkin,
        tier: Rarity::Mythical,
        chroma: true,
        droppable: false,
        edition: crate::editions::ALPHA,
        smith: CREYON,
        description: "A pale blade of the Force's spirits: a glass rod of moonlight brighter at its edges, slow ripples climbing it, cold spirit mist sinking to the hilt and faint echoes of the blade hanging in the air.",
        how_to_get: FROM_THE_TEAM,
    },
];

/// The unlockable `id` names (spaces round it and case ignored), if this client knows it.
pub(crate) fn find(id: &str) -> Option<&'static Unlockable> {
    let id = id.trim();
    ALL.iter()
        .find(|unlockable| unlockable.id.eq_ignore_ascii_case(id))
}

/// The blade skin `id` names, if this client knows one by it.
pub(crate) fn blade_skin(id: &str) -> Option<&'static Unlockable> {
    find(id).filter(|unlockable| unlockable.is_blade_skin())
}

/// Whether the blade skin `id` names is a chroma ([`Unlockable::chroma`]).
pub(crate) fn is_chroma(id: &str) -> bool {
    blade_skin(id).is_some_and(|unlockable| unlockable.chroma)
}

/// The blade skins, in catalogue order.
pub(crate) fn blade_skins() -> impl Iterator<Item = &'static Unlockable> {
    ALL.iter().filter(|unlockable| unlockable.is_blade_skin())
}

/// The blade skins as players see them listed: the rarest tier first, then in
/// catalogue order within a tier.
pub(crate) fn blade_skins_by_tier() -> &'static [&'static Unlockable] {
    static ORDER: std::sync::OnceLock<Vec<&'static Unlockable>> = std::sync::OnceLock::new();
    ORDER.get_or_init(|| {
        let mut skins: Vec<_> = blade_skins().collect();
        // A stable sort keeps the catalogue's order within a tier.
        skins.sort_by_key(|skin| std::cmp::Reverse(skin.tier));
        skins
    })
}

/// Wear blade skin `id`, `""` for the stock blade: what the Collection's Equip and
/// Unequip and the Saber tab's blade choice do.
pub(crate) fn wear(console: &mut crate::console::ViewerConsole, id: &str) {
    console.set_cvar(SABER_SKIN_CVAR, id);
}

/// What this client knows of the unlocks the player's own hub profile lists, by the
/// same rule as the gate on what they wear (`player_identity::owns_unlock`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Holdings<'a> {
    /// The identity is off: unlockables need it.
    IdentityOff,
    /// No hub is set (`cl_hubUrl`).
    NoHub,
    /// The hub has not answered yet, or cannot be reached.
    Waiting,
    /// The own profile's unlocks.
    Known(&'a [sjk_identity::Unlock]),
}

impl<'a> Holdings<'a> {
    /// From `cl_identity` (`enabled`) and the identity service's snapshot.
    pub(crate) fn of(enabled: bool, snapshot: Option<&'a sjk_identity::Snapshot>) -> Self {
        use sjk_identity::Status;
        let Some(snapshot) = snapshot.filter(|_| enabled) else {
            return Self::IdentityOff;
        };
        match (&snapshot.status, &snapshot.me) {
            (Status::Disabled, _) => Self::IdentityOff,
            (Status::NoHub, _) => Self::NoHub,
            (_, Some(me)) => Self::Known(&me.unlocks),
            (_, None) => Self::Waiting,
        }
    }

    /// The grant of `id` the profile lists, if it lists it.
    pub(crate) fn unlock(&self, id: &str) -> Option<&'a sjk_identity::Unlock> {
        match self {
            Self::Known(unlocks) => unlocks.iter().find(|unlock| unlock.id == id),
            _ => None,
        }
    }

    /// Whether the player may wear `unlockable`: a blade skin their profile lists. The
    /// Collection's Equip and the Saber tab's blade choice offer these.
    pub(crate) fn can_wear(&self, unlockable: &Unlockable) -> bool {
        unlockable.is_blade_skin() && self.unlock(unlockable.id).is_some()
    }

    /// The blade skin the player wears: the one `setting` (`cg_saberSkin`) names, while
    /// their profile lists it; `None` for the stock blade.
    pub(crate) fn worn_blade_skin(&self, setting: &str) -> Option<&'static Unlockable> {
        blade_skin(setting).filter(|skin| self.can_wear(skin))
    }

    /// Why what the player holds is not known, in a sentence; `None` once it is.
    pub(crate) fn reason(&self) -> Option<&'static str> {
        match self {
            Self::IdentityOff => {
                Some("Unlockables need the SJK identity: switch it on in Identity settings.")
            }
            Self::NoHub => Some("No hub is set: unlockables are kept on the SJK hub (cl_hubUrl)."),
            Self::Waiting => Some("Waiting for the SJK hub to answer."),
            Self::Known(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_chromas_are_the_skins_drawn_in_any_colour() {
        let chromas: Vec<_> = super::blade_skins()
            .filter(|unlockable| unlockable.chroma)
            .map(|unlockable| unlockable.id)
            .collect();
        assert_eq!(
            chromas,
            [
                "saber_storm",
                "saber_unstable",
                "saber_spectral",
                "saber_glitch",
                "saber_hologram",
                "saber_runic",
                "saber_heartbeat",
                "saber_neon",
                // Creyon's Blade Lab designs that take a colour (blade_lab_designs.rs): the
                // parts that make sense coloured, the rest kept (the red corruption, the
                // snake, the butterflies, the overheat).
                "saber_peppermint",
                "saber_bubble",
                "saber_lavalamp",
                "saber_reactor",
                "saber_sketch",
                "saber_hourglass",
                "saber_serpent",
                "saber_monarch",
                "saber_comic",
                // Blade Lab signatures drawn in the palette's colours (blade_signatures.rs).
                "saber_abyssal_eye",
                "saber_aurora_wyrm",
                "saber_between_worlds",
                "saber_big_bang",
                "saber_carbonite",
                "saber_dyson_sphere",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_hyperspace",
                "saber_kyber_heart",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_star_dragon",
                "saber_starforge",
                "saber_superlaser",
                "saber_uneti_tree",
                "saber_witchfire",
                "saber_wraithlight",
            ]
        );
        assert!(super::is_chroma("SABER_GLITCH"));
        assert!(!super::is_chroma("saber_sun"));
        assert!(!super::is_chroma("saber_nothing"));
    }

    use super::*;

    /// Creyon's "Saber ideas" page gave the 21 plain ones (14/10/2026) and his Blade Lab
    /// page the ones after them (15/10/2026); Sol the others.
    #[test]
    fn every_shader_names_its_saber_smith() {
        let creyon: Vec<_> = blade_skins()
            .filter(|skin| skin.smith == CREYON)
            .map(|skin| skin.id)
            .collect();
        assert_eq!(
            creyon,
            [
                "saber_tidal",
                "saber_circuit",
                "saber_aurora",
                "saber_echo",
                "saber_nebula",
                "saber_cinder",
                "saber_venom",
                "saber_sakura",
                "saber_firefly",
                "saber_slate",
                "saber_amber",
                "saber_lilac",
                "saber_mint",
                "saber_coral",
                "saber_ivory",
                "saber_glacier",
                "saber_dusk",
                "saber_ruby",
                "saber_amethyst",
                "saber_jade",
                "saber_moonlight",
                "saber_neon",
                "saber_peppermint",
                "saber_bubble",
                "saber_lavalamp",
                "saber_reactor",
                "saber_bleeding",
                "saber_sketch",
                "saber_hourglass",
                "saber_fiber",
                "saber_serpent",
                "saber_monarch",
                "saber_phoenix",
                "saber_quantum",
                "saber_comic",
                "saber_quicksilver",
                "saber_obsidian",
                // His Blade Lab's signature shaders (19/10/2026).
                "saber_abyssal_eye",
                "saber_aurora_wyrm",
                "saber_balance",
                "saber_between_worlds",
                "saber_big_bang",
                "saber_carbonite",
                "saber_dyson_sphere",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_exogorth",
                "saber_hyperspace",
                "saber_kyber_heart",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_sith_holocron",
                "saber_star_dragon",
                "saber_starforge",
                "saber_superlaser",
                "saber_uneti_tree",
                "saber_witchfire",
                "saber_world_shift",
                "saber_wraithlight",
            ]
        );
        assert!(blade_skins().all(|skin| [SOL, CREYON].contains(&skin.smith)));
    }

    /// Sol's tiers (10/10/2026); Prism, Glitch and Banner are placed provisionally.
    #[test]
    fn every_shader_has_its_tier_and_the_rarest_list_first() {
        let of = |rarity: Rarity| -> Vec<&str> {
            blade_skins()
                .filter(|skin| skin.tier == rarity)
                .map(|skin| skin.id)
                .collect()
        };
        assert_eq!(
            of(Rarity::Mythical),
            [
                "saber_sun",
                "saber_void",
                "saber_molten",
                "saber_spectral",
                "saber_reactor",
                "saber_bleeding",
                "saber_serpent",
                "saber_monarch",
                "saber_phoenix",
                "saber_quantum",
                "saber_obsidian",
                // Creyon's Blade Lab signatures, mythicals first (19/10/2026).
                "saber_abyssal_eye",
                "saber_aurora_wyrm",
                "saber_balance",
                "saber_between_worlds",
                "saber_big_bang",
                "saber_carbonite",
                "saber_dyson_sphere",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_exogorth",
                "saber_hyperspace",
                "saber_kyber_heart",
                "saber_nebula_song",
                "saber_quasar",
                "saber_singing_kyber",
                "saber_sith_holocron",
                "saber_star_dragon",
                "saber_starforge",
                "saber_superlaser",
                "saber_uneti_tree",
                "saber_witchfire",
                "saber_world_shift",
                "saber_wraithlight",
            ]
        );
        assert_eq!(
            of(Rarity::Legendary),
            [
                "saber_storm",
                "saber_prism",
                "saber_unstable",
                "saber_chameleon",
                "saber_hourglass",
                "saber_fiber",
                "saber_comic",
                "saber_quicksilver",
            ]
        );
        assert_eq!(
            of(Rarity::Rare),
            [
                "saber_frost",
                "saber_glitch",
                "saber_hologram",
                "saber_runic",
                "saber_banner",
                "saber_heartbeat",
                "saber_bubble",
                "saber_lavalamp",
                "saber_sketch",
            ]
        );
        assert_eq!(
            of(Rarity::Uncommon),
            [
                "saber_nebula",
                "saber_cinder",
                "saber_venom",
                "saber_sakura",
                "saber_firefly",
                "saber_glacier",
                "saber_dusk",
                "saber_ruby",
                "saber_amethyst",
                "saber_jade",
                "saber_moonlight",
                "saber_neon",
                "saber_peppermint",
            ]
        );
        assert_eq!(
            of(Rarity::Common),
            [
                "saber_tidal",
                "saber_circuit",
                "saber_aurora",
                "saber_echo",
                "saber_slate",
                "saber_amber",
                "saber_lilac",
                "saber_mint",
                "saber_coral",
                "saber_ivory",
            ]
        );
        let order = blade_skins_by_tier();
        assert_eq!(order.len(), blade_skins().count());
        assert!(order.windows(2).all(|pair| pair[0].tier >= pair[1].tier));
        assert_eq!(order[0].id, "saber_sun");
        assert_eq!(order[34].id, "saber_storm");
        // The holocrons' names and colours.
        assert_eq!(Rarity::Mythical.label(), "Mythical");
        assert_eq!(Rarity::Uncommon.label(), "Uncommon");
        assert_eq!(Rarity::Common.label(), "Common");
        assert_eq!(Rarity::Rare.colour(), crate::holocrons::TIERS[1].colour);
        assert_eq!(Rarity::Common.symbol(), crate::symbols::RARITY[0]);
        assert_eq!(Rarity::Mythical.symbol(), crate::symbols::RARITY[4]);
    }

    /// Sol's rule (17/10/2026): every shader drops from holocrons but those a medal
    /// brings (no achievement brings one yet), and its "How to get it" says which. The
    /// Blade Lab signatures are kept out too while they are tried (`FROM_THE_TEAM`).
    #[test]
    fn every_shader_but_the_medals_drops() {
        for skin in blade_skins() {
            let medal = crate::medals::Medal::ALL
                .iter()
                .any(|medal| medal.blade() == Some(skin.id));
            // A new signature on trial (the reworked designs drop as before).
            let trying = skin.how_to_get == FROM_THE_TEAM;
            assert!(
                !trying || crate::blade_signatures::of(skin.id).is_some(),
                "{}",
                skin.id
            );
            assert_eq!(
                skin.droppable,
                !medal && !trying,
                "{}: a medal's shader and a signature on trial never drop, every other one does today",
                skin.id
            );
            assert_eq!(
                skin.how_to_get == FROM_HOLOCRONS,
                skin.droppable,
                "{}",
                skin.id
            );
        }
    }

    #[test]
    fn ids_are_unique_and_every_blade_skin_has_one_entry() {
        for (index, unlockable) in ALL.iter().enumerate() {
            assert!(
                ALL[index + 1..]
                    .iter()
                    .all(|other| !other.id.eq_ignore_ascii_case(unlockable.id)),
                "{} is listed twice",
                unlockable.id
            );
            assert!(
                unlockable
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{} is a hub id",
                unlockable.id
            );
            assert!(!unlockable.name.is_empty() && !unlockable.description.is_empty());
            assert!(unlockable.how_to_get.ends_with('.'));
        }
        assert_eq!(
            blade_skins().count(),
            ALL.iter().filter(|u| u.kind == Kind::BladeSkin).count()
        );
    }

    #[test]
    fn holdings_follow_the_identity_and_the_own_profile() {
        use sjk_identity::{Profile, Snapshot, Status, Unlock};
        let unlock = Unlock {
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "Thanks".into(),
            count: 1,
            medal: None,
        };
        let me = Profile {
            credits: 0,
            cards: Vec::new(),
            keys: Vec::new(),
            key_id: "0123456789abcdef".into(),
            key: String::new(),
            name: "Sol".into(),
            bio: String::new(),
            verified: false,
            staff: false,
            trust: None,
            emoji: String::new(),
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            unlocks: vec![unlock.clone()],
            avatar: String::new(),
            holocron_counts: Default::default(),
            holocrons: Vec::new(),
            shards: 0,
        };
        let snapshot = |status, me| Snapshot {
            loot: Vec::new(),
            opening: None,
            reveal: None,
            local_key_id: "0123456789abcdef".into(),
            status,
            key_id: "0123456789abcdef".into(),
            me,
            server: None,
            players: Vec::new(),
            profiles: std::collections::HashMap::new(),
            model_checks: std::collections::HashMap::new(),
            profiles_revision: 0,
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            assets_note: None,
            crashes_sent: 0,
            crash_note: None,
            holocrons: None,
            log_requests: Vec::new(),
            log_answer: None,
            economy: None,
            shop: None,
            trade: None,
        };
        let online = snapshot(Status::Online, Some(me));
        assert_eq!(Holdings::of(false, Some(&online)), Holdings::IdentityOff);
        assert_eq!(Holdings::of(true, None), Holdings::IdentityOff);
        let known = Holdings::of(true, Some(&online));
        assert_eq!(known.unlock("saber_sun"), Some(&unlock));
        assert_eq!(known.unlock("saber_moon"), None);
        assert_eq!(known.reason(), None);
        let waiting = snapshot(Status::Registering, None);
        assert_eq!(Holdings::of(true, Some(&waiting)), Holdings::Waiting);
        let no_hub = snapshot(Status::NoHub, None);
        assert_eq!(Holdings::of(true, Some(&no_hub)), Holdings::NoHub);
        assert!(
            Holdings::IdentityOff
                .reason()
                .unwrap()
                .contains("SJK identity")
        );
        assert_eq!(Holdings::Waiting.unlock("saber_sun"), None);
    }

    /// The hub's catalogue (`src/unlocks.rs` in Sol-Vulpes/SJK-hub) lists exactly these, in
    /// this order and with these names; its test pins the same list, so a change to one
    /// side fails until the other follows.
    #[test]
    fn the_catalogue_matches_the_hubs() {
        let listed: Vec<(&str, &str)> = ALL.iter().map(|u| (u.id, u.name)).collect();
        assert_eq!(
            listed,
            [
                ("saber_sun", "Sun blade"),
                ("saber_storm", "Storm blade"),
                ("saber_void", "Void blade"),
                ("saber_frost", "Frost blade"),
                ("saber_prism", "Prism blade"),
                ("saber_unstable", "Unstable blade"),
                ("saber_molten", "Molten blade"),
                ("saber_spectral", "Spectral blade"),
                ("saber_glitch", "Glitch blade"),
                ("saber_hologram", "Hologram blade"),
                ("saber_runic", "Runic blade"),
                ("saber_chameleon", "Chameleon blade"),
                ("saber_banner", "Banner blade"),
                ("saber_heartbeat", "Heartbeat blade"),
                ("saber_tidal", "Kamino blade"),
                ("saber_circuit", "Droid Logic blade"),
                ("saber_aurora", "Hoth Aurora blade"),
                ("saber_echo", "Echo Base blade"),
                ("saber_nebula", "Nebula blade"),
                ("saber_cinder", "Mustafar Ember blade"),
                ("saber_venom", "Geonosis Acid blade"),
                ("saber_sakura", "Naboo Blossom blade"),
                ("saber_firefly", "Endor Lights blade"),
                ("saber_slate", "Durasteel blade"),
                ("saber_amber", "Tatooine Gold blade"),
                ("saber_lilac", "Lothal Twilight blade"),
                ("saber_mint", "Yavin Jungle blade"),
                ("saber_coral", "Mon Cala Coral blade"),
                ("saber_ivory", "Jedi Temple blade"),
                ("saber_glacier", "Ilum blade"),
                ("saber_dusk", "Binary Sunset blade"),
                ("saber_ruby", "Sith Ruby blade"),
                ("saber_amethyst", "Kyber Amethyst blade"),
                ("saber_jade", "Dagobah Jade blade"),
                ("saber_moonlight", "Ahch-To Moonlight blade"),
                ("saber_neon", "Coruscant Neon blade"),
                ("saber_peppermint", "Kessel Spice blade"),
                ("saber_bubble", "Bacta blade"),
                ("saber_lavalamp", "Coaxium blade"),
                ("saber_reactor", "Reactor blade"),
                ("saber_bleeding", "Bleeding Kyber blade"),
                ("saber_sketch", "Death Star Plans blade"),
                ("saber_hourglass", "Beskar Forge blade"),
                ("saber_fiber", "Kyber Cluster blade"),
                ("saber_serpent", "Sith Alchemy blade"),
                ("saber_monarch", "Felucia Spores blade"),
                ("saber_phoenix", "Phoenix Squadron blade"),
                ("saber_quantum", "Mortis blade"),
                ("saber_comic", "Clone Wars blade"),
                ("saber_quicksilver", "Liquid Beskar blade"),
                ("saber_obsidian", "Obsidian Edge blade"),
                // Not in the hub yet: its catalogue follows when the team adds them.
                ("saber_abyssal_eye", "Abyssal Eye blade"),
                ("saber_aurora_wyrm", "Ilum Aurora Storm blade"),
                ("saber_balance", "Balance blade"),
                ("saber_between_worlds", "Between Worlds blade"),
                ("saber_big_bang", "Big Bang blade"),
                ("saber_carbonite", "Carbonite blade"),
                ("saber_dyson_sphere", "Dyson Sphere blade"),
                ("saber_eclipse", "Eclipse blade"),
                ("saber_event_horizon", "Event Horizon blade"),
                ("saber_exogorth", "Exogorth blade"),
                ("saber_hyperspace", "Hyperspace blade"),
                ("saber_kyber_heart", "Kyber Heart blade"),
                ("saber_nebula_song", "Nebula Song blade"),
                ("saber_quasar", "Quasar blade"),
                ("saber_singing_kyber", "Singing Kyber blade"),
                ("saber_sith_holocron", "Sith Holocron blade"),
                ("saber_star_dragon", "Star Dragon blade"),
                ("saber_starforge", "Starforge blade"),
                ("saber_superlaser", "Superlaser blade"),
                ("saber_uneti_tree", "Uneti Tree blade"),
                ("saber_witchfire", "Witchfire blade"),
                ("saber_world_shift", "World Shift blade"),
                ("saber_wraithlight", "Wraithlight blade"),
            ]
        );
        assert!(ALL.iter().all(Unlockable::is_blade_skin));
        // The blade skins fit the renderer's slots, every one loaded at once.
        assert!(blade_skins().count() <= crate::saber_skins::MAX_SKINS);
    }

    #[test]
    fn ids_are_found_whatever_the_case_and_spaces() {
        assert_eq!(find("saber_sun").map(|u| u.name), Some("Sun blade"));
        assert_eq!(blade_skin(" SABER_Sun ").map(|u| u.id), Some("saber_sun"));
        assert!(blade_skin("saber_sun").is_some_and(Unlockable::is_blade_skin));
        assert!(find("saber_moon").is_none());
        assert!(find("").is_none());
    }
}

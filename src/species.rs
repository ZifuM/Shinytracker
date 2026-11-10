//! National Dex species list (Gen 1–9) plus helpers for sprite slugs and
//! OCR matching keys. No external dependencies, so it can be unit tested anywhere.

/// All species in National Dex order, one per line.
const SPECIES_RAW: &str = "\
Bulbasaur\nIvysaur\nVenusaur\nCharmander\nCharmeleon\nCharizard\nSquirtle\nWartortle\nBlastoise\nCaterpie\nMetapod\nButterfree\nWeedle\nKakuna\nBeedrill\nPidgey\nPidgeotto\nPidgeot\nRattata\nRaticate\nSpearow\nFearow\nEkans\nArbok\nPikachu\nRaichu\nSandshrew\nSandslash\nNidoran♀\nNidorina\nNidoqueen\nNidoran♂\nNidorino\nNidoking\nClefairy\nClefable\nVulpix\nNinetales\nJigglypuff\nWigglytuff\nZubat\nGolbat\nOddish\nGloom\nVileplume\nParas\nParasect\nVenonat\nVenomoth\nDiglett\nDugtrio\nMeowth\nPersian\nPsyduck\nGolduck\nMankey\nPrimeape\nGrowlithe\nArcanine\nPoliwag\nPoliwhirl\nPoliwrath\nAbra\nKadabra\nAlakazam\nMachop\nMachoke\nMachamp\nBellsprout\nWeepinbell\nVictreebel\nTentacool\nTentacruel\nGeodude\nGraveler\nGolem\nPonyta\nRapidash\nSlowpoke\nSlowbro\nMagnemite\nMagneton\nFarfetch'd\nDoduo\nDodrio\nSeel\nDewgong\nGrimer\nMuk\nShellder\nCloyster\nGastly\nHaunter\nGengar\nOnix\nDrowzee\nHypno\nKrabby\nKingler\nVoltorb\nElectrode\nExeggcute\nExeggutor\nCubone\nMarowak\nHitmonlee\nHitmonchan\nLickitung\nKoffing\nWeezing\nRhyhorn\nRhydon\nChansey\nTangela\nKangaskhan\nHorsea\nSeadra\nGoldeen\nSeaking\nStaryu\nStarmie\nMr. Mime\nScyther\nJynx\nElectabuzz\nMagmar\nPinsir\nTauros\nMagikarp\nGyarados\nLapras\nDitto\nEevee\nVaporeon\nJolteon\nFlareon\nPorygon\nOmanyte\nOmastar\nKabuto\nKabutops\nAerodactyl\nSnorlax\nArticuno\nZapdos\nMoltres\nDratini\nDragonair\nDragonite\nMewtwo\nMew\n\
Chikorita\nBayleef\nMeganium\nCyndaquil\nQuilava\nTyphlosion\nTotodile\nCroconaw\nFeraligatr\nSentret\nFurret\nHoothoot\nNoctowl\nLedyba\nLedian\nSpinarak\nAriados\nCrobat\nChinchou\nLanturn\nPichu\nCleffa\nIgglybuff\nTogepi\nTogetic\nNatu\nXatu\nMareep\nFlaaffy\nAmpharos\nBellossom\nMarill\nAzumarill\nSudowoodo\nPolitoed\nHoppip\nSkiploom\nJumpluff\nAipom\nSunkern\nSunflora\nYanma\nWooper\nQuagsire\nEspeon\nUmbreon\nMurkrow\nSlowking\nMisdreavus\nUnown\nWobbuffet\nGirafarig\nPineco\nForretress\nDunsparce\nGligar\nSteelix\nSnubbull\nGranbull\nQwilfish\nScizor\nShuckle\nHeracross\nSneasel\nTeddiursa\nUrsaring\nSlugma\nMagcargo\nSwinub\nPiloswine\nCorsola\nRemoraid\nOctillery\nDelibird\nMantine\nSkarmory\nHoundour\nHoundoom\nKingdra\nPhanpy\nDonphan\nPorygon2\nStantler\nSmeargle\nTyrogue\nHitmontop\nSmoochum\nElekid\nMagby\nMiltank\nBlissey\nRaikou\nEntei\nSuicune\nLarvitar\nPupitar\nTyranitar\nLugia\nHo-Oh\nCelebi\n\
Treecko\nGrovyle\nSceptile\nTorchic\nCombusken\nBlaziken\nMudkip\nMarshtomp\nSwampert\nPoochyena\nMightyena\nZigzagoon\nLinoone\nWurmple\nSilcoon\nBeautifly\nCascoon\nDustox\nLotad\nLombre\nLudicolo\nSeedot\nNuzleaf\nShiftry\nTaillow\nSwellow\nWingull\nPelipper\nRalts\nKirlia\nGardevoir\nSurskit\nMasquerain\nShroomish\nBreloom\nSlakoth\nVigoroth\nSlaking\nNincada\nNinjask\nShedinja\nWhismur\nLoudred\nExploud\nMakuhita\nHariyama\nAzurill\nNosepass\nSkitty\nDelcatty\nSableye\nMawile\nAron\nLairon\nAggron\nMeditite\nMedicham\nElectrike\nManectric\nPlusle\nMinun\nVolbeat\nIllumise\nRoselia\nGulpin\nSwalot\nCarvanha\nSharpedo\nWailmer\nWailord\nNumel\nCamerupt\nTorkoal\nSpoink\nGrumpig\nSpinda\nTrapinch\nVibrava\nFlygon\nCacnea\nCacturne\nSwablu\nAltaria\nZangoose\nSeviper\nLunatone\nSolrock\nBarboach\nWhiscash\nCorphish\nCrawdaunt\nBaltoy\nClaydol\nLileep\nCradily\nAnorith\nArmaldo\nFeebas\nMilotic\nCastform\nKecleon\nShuppet\nBanette\nDuskull\nDusclops\nTropius\nChimecho\nAbsol\nWynaut\nSnorunt\nGlalie\nSpheal\nSealeo\nWalrein\nClamperl\nHuntail\nGorebyss\nRelicanth\nLuvdisc\nBagon\nShelgon\nSalamence\nBeldum\nMetang\nMetagross\nRegirock\nRegice\nRegisteel\nLatias\nLatios\nKyogre\nGroudon\nRayquaza\nJirachi\nDeoxys\n\
Turtwig\nGrotle\nTorterra\nChimchar\nMonferno\nInfernape\nPiplup\nPrinplup\nEmpoleon\nStarly\nStaravia\nStaraptor\nBidoof\nBibarel\nKricketot\nKricketune\nShinx\nLuxio\nLuxray\nBudew\nRoserade\nCranidos\nRampardos\nShieldon\nBastiodon\nBurmy\nWormadam\nMothim\nCombee\nVespiquen\nPachirisu\nBuizel\nFloatzel\nCherubi\nCherrim\nShellos\nGastrodon\nAmbipom\nDrifloon\nDrifblim\nBuneary\nLopunny\nMismagius\nHonchkrow\nGlameow\nPurugly\nChingling\nStunky\nSkuntank\nBronzor\nBronzong\nBonsly\nMime Jr.\nHappiny\nChatot\nSpiritomb\nGible\nGabite\nGarchomp\nMunchlax\nRiolu\nLucario\nHippopotas\nHippowdon\nSkorupi\nDrapion\nCroagunk\nToxicroak\nCarnivine\nFinneon\nLumineon\nMantyke\nSnover\nAbomasnow\nWeavile\nMagnezone\nLickilicky\nRhyperior\nTangrowth\nElectivire\nMagmortar\nTogekiss\nYanmega\nLeafeon\nGlaceon\nGliscor\nMamoswine\nPorygon-Z\nGallade\nProbopass\nDusknoir\nFroslass\nRotom\nUxie\nMesprit\nAzelf\nDialga\nPalkia\nHeatran\nRegigigas\nGiratina\nCresselia\nPhione\nManaphy\nDarkrai\nShaymin\nArceus\n\
Victini\nSnivy\nServine\nSerperior\nTepig\nPignite\nEmboar\nOshawott\nDewott\nSamurott\nPatrat\nWatchog\nLillipup\nHerdier\nStoutland\nPurrloin\nLiepard\nPansage\nSimisage\nPansear\nSimisear\nPanpour\nSimipour\nMunna\nMusharna\nPidove\nTranquill\nUnfezant\nBlitzle\nZebstrika\nRoggenrola\nBoldore\nGigalith\nWoobat\nSwoobat\nDrilbur\nExcadrill\nAudino\nTimburr\nGurdurr\nConkeldurr\nTympole\nPalpitoad\nSeismitoad\nThroh\nSawk\nSewaddle\nSwadloon\nLeavanny\nVenipede\nWhirlipede\nScolipede\nCottonee\nWhimsicott\nPetilil\nLilligant\nBasculin\nSandile\nKrokorok\nKrookodile\nDarumaka\nDarmanitan\nMaractus\nDwebble\nCrustle\nScraggy\nScrafty\nSigilyph\nYamask\nCofagrigus\nTirtouga\nCarracosta\nArchen\nArcheops\nTrubbish\nGarbodor\nZorua\nZoroark\nMinccino\nCinccino\nGothita\nGothorita\nGothitelle\nSolosis\nDuosion\nReuniclus\nDucklett\nSwanna\nVanillite\nVanillish\nVanilluxe\nDeerling\nSawsbuck\nEmolga\nKarrablast\nEscavalier\nFoongus\nAmoonguss\nFrillish\nJellicent\nAlomomola\nJoltik\nGalvantula\nFerroseed\nFerrothorn\nKlink\nKlang\nKlinklang\nTynamo\nEelektrik\nEelektross\nElgyem\nBeheeyem\nLitwick\nLampent\nChandelure\nAxew\nFraxure\nHaxorus\nCubchoo\nBeartic\nCryogonal\nShelmet\nAccelgor\nStunfisk\nMienfoo\nMienshao\nDruddigon\nGolett\nGolurk\nPawniard\nBisharp\nBouffalant\nRufflet\nBraviary\nVullaby\nMandibuzz\nHeatmor\nDurant\nDeino\nZweilous\nHydreigon\nLarvesta\nVolcarona\nCobalion\nTerrakion\nVirizion\nTornadus\nThundurus\nReshiram\nZekrom\nLandorus\nKyurem\nKeldeo\nMeloetta\nGenesect\n\
Chespin\nQuilladin\nChesnaught\nFennekin\nBraixen\nDelphox\nFroakie\nFrogadier\nGreninja\nBunnelby\nDiggersby\nFletchling\nFletchinder\nTalonflame\nScatterbug\nSpewpa\nVivillon\nLitleo\nPyroar\nFlabébé\nFloette\nFlorges\nSkiddo\nGogoat\nPancham\nPangoro\nFurfrou\nEspurr\nMeowstic\nHonedge\nDoublade\nAegislash\nSpritzee\nAromatisse\nSwirlix\nSlurpuff\nInkay\nMalamar\nBinacle\nBarbaracle\nSkrelp\nDragalge\nClauncher\nClawitzer\nHelioptile\nHeliolisk\nTyrunt\nTyrantrum\nAmaura\nAurorus\nSylveon\nHawlucha\nDedenne\nCarbink\nGoomy\nSliggoo\nGoodra\nKlefki\nPhantump\nTrevenant\nPumpkaboo\nGourgeist\nBergmite\nAvalugg\nNoibat\nNoivern\nXerneas\nYveltal\nZygarde\nDiancie\nHoopa\nVolcanion\n\
Rowlet\nDartrix\nDecidueye\nLitten\nTorracat\nIncineroar\nPopplio\nBrionne\nPrimarina\nPikipek\nTrumbeak\nToucannon\nYungoos\nGumshoos\nGrubbin\nCharjabug\nVikavolt\nCrabrawler\nCrabominable\nOricorio\nCutiefly\nRibombee\nRockruff\nLycanroc\nWishiwashi\nMareanie\nToxapex\nMudbray\nMudsdale\nDewpider\nAraquanid\nFomantis\nLurantis\nMorelull\nShiinotic\nSalandit\nSalazzle\nStufful\nBewear\nBounsweet\nSteenee\nTsareena\nComfey\nOranguru\nPassimian\nWimpod\nGolisopod\nSandygast\nPalossand\nPyukumuku\nType: Null\nSilvally\nMinior\nKomala\nTurtonator\nTogedemaru\nMimikyu\nBruxish\nDrampa\nDhelmise\nJangmo-o\nHakamo-o\nKommo-o\nTapu Koko\nTapu Lele\nTapu Bulu\nTapu Fini\nCosmog\nCosmoem\nSolgaleo\nLunala\nNihilego\nBuzzwole\nPheromosa\nXurkitree\nCelesteela\nKartana\nGuzzlord\nNecrozma\nMagearna\nMarshadow\nPoipole\nNaganadel\nStakataka\nBlacephalon\nZeraora\nMeltan\nMelmetal\n\
Grookey\nThwackey\nRillaboom\nScorbunny\nRaboot\nCinderace\nSobble\nDrizzile\nInteleon\nSkwovet\nGreedent\nRookidee\nCorvisquire\nCorviknight\nBlipbug\nDottler\nOrbeetle\nNickit\nThievul\nGossifleur\nEldegoss\nWooloo\nDubwool\nChewtle\nDrednaw\nYamper\nBoltund\nRolycoly\nCarkol\nCoalossal\nApplin\nFlapple\nAppletun\nSilicobra\nSandaconda\nCramorant\nArrokuda\nBarraskewda\nToxel\nToxtricity\nSizzlipede\nCentiskorch\nClobbopus\nGrapploct\nSinistea\nPolteageist\nHatenna\nHattrem\nHatterene\nImpidimp\nMorgrem\nGrimmsnarl\nObstagoon\nPerrserker\nCursola\nSirfetch'd\nMr. Rime\nRunerigus\nMilcery\nAlcremie\nFalinks\nPincurchin\nSnom\nFrosmoth\nStonjourner\nEiscue\nIndeedee\nMorpeko\nCufant\nCopperajah\nDracozolt\nArctozolt\nDracovish\nArctovish\nDuraludon\nDreepy\nDrakloak\nDragapult\nZacian\nZamazenta\nEternatus\nKubfu\nUrshifu\nZarude\nRegieleki\nRegidrago\nGlastrier\nSpectrier\nCalyrex\nWyrdeer\nKleavor\nUrsaluna\nBasculegion\nSneasler\nOverqwil\nEnamorus\n\
Sprigatito\nFloragato\nMeowscarada\nFuecoco\nCrocalor\nSkeledirge\nQuaxly\nQuaxwell\nQuaquaval\nLechonk\nOinkologne\nTarountula\nSpidops\nNymble\nLokix\nPawmi\nPawmo\nPawmot\nTandemaus\nMaushold\nFidough\nDachsbun\nSmoliv\nDolliv\nArboliva\nSquawkabilly\nNacli\nNaclstack\nGarganacl\nCharcadet\nArmarouge\nCeruledge\nTadbulb\nBellibolt\nWattrel\nKilowattrel\nMaschiff\nMabosstiff\nShroodle\nGrafaiai\nBramblin\nBrambleghast\nToedscool\nToedscruel\nKlawf\nCapsakid\nScovillain\nRellor\nRabsca\nFlittle\nEspathra\nTinkatink\nTinkatuff\nTinkaton\nWiglett\nWugtrio\nBombirdier\nFinizen\nPalafin\nVaroom\nRevavroom\nCyclizar\nOrthworm\nGlimmet\nGlimmora\nGreavard\nHoundstone\nFlamigo\nCetoddle\nCetitan\nVeluza\nDondozo\nTatsugiri\nAnnihilape\nClodsire\nFarigiraf\nDudunsparce\nKingambit\nGreat Tusk\nScream Tail\nBrute Bonnet\nFlutter Mane\nSlither Wing\nSandy Shocks\nIron Treads\nIron Bundle\nIron Hands\nIron Jugulis\nIron Moth\nIron Thorns\nFrigibax\nArctibax\nBaxcalibur\nGimmighoul\nGholdengo\nWo-Chien\nChien-Pao\nTing-Lu\nChi-Yu\nRoaring Moon\nIron Valiant\nKoraidon\nMiraidon\nWalking Wake\nIron Leaves\nDipplin\nPoltchageist\nSinistcha\nOkidogi\nMunkidori\nFezandipiti\nOgerpon\nArchaludon\nHydrapple\nGouging Fire\nRaging Bolt\nIron Boulder\nIron Crown\nTerapagos\nPecharunt";

/// Species whose default sprite on pokemondb lives under a form-suffixed slug.
/// The plain slug is always tried as well, so these are just extra guesses.
const FORM_SLUGS: &[(&str, &[&str])] = &[
    ("deoxys", &["deoxys-normal"]),
    ("burmy", &["burmy-plant"]),
    ("wormadam", &["wormadam-plant"]),
    ("shellos", &["shellos-west"]),
    ("gastrodon", &["gastrodon-west"]),
    ("giratina", &["giratina-altered"]),
    ("shaymin", &["shaymin-land"]),
    ("basculin", &["basculin-red-striped"]),
    ("darmanitan", &["darmanitan-standard"]),
    ("tornadus", &["tornadus-incarnate"]),
    ("thundurus", &["thundurus-incarnate"]),
    ("landorus", &["landorus-incarnate"]),
    ("keldeo", &["keldeo-ordinary"]),
    ("meloetta", &["meloetta-aria"]),
    ("meowstic", &["meowstic-male"]),
    ("aegislash", &["aegislash-shield"]),
    ("pumpkaboo", &["pumpkaboo-average"]),
    ("gourgeist", &["gourgeist-average"]),
    ("zygarde", &["zygarde-50"]),
    ("hoopa", &["hoopa-confined"]),
    ("oricorio", &["oricorio-baile"]),
    ("lycanroc", &["lycanroc-midday"]),
    ("wishiwashi", &["wishiwashi-solo"]),
    ("minior", &["minior-meteor"]),
    ("mimikyu", &["mimikyu-disguised"]),
    ("toxtricity", &["toxtricity-amped"]),
    ("eiscue", &["eiscue-ice"]),
    ("indeedee", &["indeedee-male"]),
    ("morpeko", &["morpeko-full-belly"]),
    ("urshifu", &["urshifu-single-strike"]),
    ("basculegion", &["basculegion-male"]),
    ("enamorus", &["enamorus-incarnate"]),
    ("oinkologne", &["oinkologne-male"]),
    ("maushold", &["maushold-family-of-four"]),
    ("squawkabilly", &["squawkabilly-green-plumage"]),
    ("palafin", &["palafin-zero"]),
    ("tatsugiri", &["tatsugiri-curly"]),
    ("dudunsparce", &["dudunsparce-two-segment"]),
];

#[derive(Debug, Clone)]
pub struct Species {
    /// Display name, e.g. "Mr. Mime".
    pub name: &'static str,
    /// pokemondb-style slug, e.g. "mr-mime".
    pub slug: String,
    /// OCR matching key: uppercase A–Z/0–9 only, e.g. "MRMIME".
    pub key: String,
    /// National Pokédex number (1-1025).
    pub dex: u32,
}

impl Species {
    /// Slugs to try, in order, when downloading a sprite.
    pub fn sprite_slugs(&self) -> Vec<String> {
        let mut v = vec![self.slug.clone()];
        if let Some((_, extra)) = FORM_SLUGS.iter().find(|(s, _)| *s == self.slug) {
            v.extend(extra.iter().map(|s| s.to_string()));
        }
        v
    }
}

pub fn slugify(name: &str) -> String {
    let mut s = String::new();
    for ch in name.chars() {
        match ch {
            '♀' => s.push_str("-f"),
            '♂' => s.push_str("-m"),
            'é' | 'É' => s.push('e'),
            '.' | '\'' | '’' | ':' => {}
            ' ' => s.push('-'),
            c => s.extend(c.to_lowercase()),
        }
    }
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

pub fn match_key(name: &str) -> String {
    name.chars()
        .map(|c| if c == 'é' || c == 'É' { 'E' } else { c })
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub fn all() -> Vec<Species> {
    SPECIES_RAW
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .enumerate()
        .map(|(i, name)| Species {
            name,
            slug: slugify(name),
            key: match_key(name),
            dex: i as u32 + 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_national_dex() {
        let all = all();
        assert_eq!(all.len(), 1025);
        assert_eq!(all[0].name, "Bulbasaur");
        assert_eq!(all[150].name, "Mew");
        assert_eq!(all[250].name, "Celebi");
        assert_eq!(all[385].name, "Deoxys");
        assert_eq!(all[447].name, "Lucario");
        assert_eq!(all[492].name, "Arceus");
        assert_eq!(all[648].name, "Genesect");
        assert_eq!(all[657].name, "Greninja");
        assert_eq!(all[720].name, "Volcanion");
        assert_eq!(all[808].name, "Melmetal");
        assert_eq!(all[904].name, "Enamorus");
        assert_eq!(all[1024].name, "Pecharunt");
    }

    #[test]
    fn slugs_match_pokemondb() {
        assert_eq!(slugify("Mr. Mime"), "mr-mime");
        assert_eq!(slugify("Nidoran♀"), "nidoran-f");
        assert_eq!(slugify("Nidoran♂"), "nidoran-m");
        assert_eq!(slugify("Farfetch'd"), "farfetchd");
        assert_eq!(slugify("Ho-Oh"), "ho-oh");
        assert_eq!(slugify("Porygon-Z"), "porygon-z");
        assert_eq!(slugify("Mime Jr."), "mime-jr");
        assert_eq!(slugify("Flabébé"), "flabebe");
        assert_eq!(slugify("Type: Null"), "type-null");
        assert_eq!(slugify("Tapu Koko"), "tapu-koko");
    }

    #[test]
    fn keys_are_mostly_unique() {
        let all = all();
        let mut keys: Vec<_> = all.iter().map(|s| s.key.clone()).collect();
        keys.sort();
        let before = keys.len();
        keys.dedup();
        // Only the two Nidoran share a key.
        assert_eq!(before - keys.len(), 1);
    }
}

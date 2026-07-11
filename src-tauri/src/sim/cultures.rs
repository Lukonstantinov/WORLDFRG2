//! Organic culture / peoples system.
//!
//! Culture "hearths" are seeded on the land and spread outward by a least-cost
//! flood-fill (cheap across open ground, dear across mountains and deserts,
//! blocked by open sea), so neighbouring settlements share a people and culture
//! borders fall on real geographic barriers. Each hearth speaks a curated
//! "language kit" (Roman, Norse, Sinitic, …) that is LIGHTLY MUTATED per world
//! (a deterministic phoneme shift) so a culture reads familiar but is never
//! identical run-to-run.
//!
//! The computed [`CultureMap`] is stored in world `metadata` and cached in a
//! process-global so the positional, deterministic name generators in `names.rs`
//! resolve the SAME culture for a cell across the settlement / political /
//! economy layers. When no map is active (old saves), the name fns fall back to
//! the legacy 4-culture grid — so existing worlds are unchanged.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::sim::world_buffer::WorldBuffer;

/// splitmix64 finalizer (deterministic, well-mixed).
pub fn hash64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    x
}
#[inline]
fn h01(x: u64) -> f32 {
    (hash64(x) >> 40) as f32 / (1u64 << 24) as f32
}

// ── Language kits ─────────────────────────────────────────────────────────────
/// Environment a kit feels "native" to — biases which kit a hearth draws.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Env { Temperate, Cold, Arid, Tropical, Maritime }

pub struct Kit {
    pub name: &'static str,           // people / culture label
    pub lang_family: &'static str,    // language family (relatedness → assimilation/borrowing)
    pub on: &'static [&'static str],  // place-name onsets
    pub mid: &'static [&'static str],
    pub end: &'static [&'static str], // place-name endings
    pub given: &'static [&'static str],
    pub family: &'static [&'static str],
    pub guild: &'static [&'static str], // guild-style words
    pub epithet: &'static [&'static str],
    pub env: Env,
}

pub const KITS: &[Kit] = &[
    // 0 Roman
    Kit { name: "Roman", lang_family: "Italic", env: Env::Temperate,
        on: &["Aqu","Por","Nov","Ver","Cas","Lav","Tarr","Bened","Aug","Salt","Ostia","Vol","Arr","Pomp","Luc","Mar"],
        mid: &["","i","e","en","el","ar"],
        end: &["ium","a","entia","onium","aria","anum","ona","ina","etia","olum","estum","icum"],
        given: &["Marcus","Lucius","Gaius","Titus","Quintus","Publius","Aulus","Servius","Decimus","Gnaeus"],
        family: &["Cassii","Valerii","Aurelii","Cornelii","Fabii","Julii","Aemilii","Flavii","Claudii","Marcii","Licinii","Domitii","Sergii","Manlii","Caecilii","Sempronii","Postumii","Hortensii","Pinarii","Furii","Atilii","Verginii","Antonii","Tullii","Calpurnii","Porcii","Junii","Livii","Octavii","Terentii","Papirii","Quinctii","Servilii","Aelii","Memmii","Mucii","Naevii","Plautii","Rutilii","Vibii","Curtii","Lutatii","Caninii","Herennii","Ostorii","Petronii"],
        guild: &["Collegium","Societas","Corpus","Negotiatores","Mercatura","Argentaria","Officina"],
        epithet: &["Magna","Augusta","Maior"] },
    // 1 Greek
    Kit { name: "Hellene", lang_family: "Hellenic", env: Env::Maritime,
        on: &["Meg","Thess","Korin","Pyr","Hali","Eph","Mil","Delph","Olymp","Argos","Naxa","Therm","Knoss","Pell","Syrak","Mykon"],
        mid: &["","a","i","o","ar","is"],
        end: &["os","on","aia","polis","andros","ia","ene","yssa","kos","thea","nthos","kleia"],
        given: &["Alexios","Theron","Nikias","Demetrios","Leonidas","Kallias","Philon","Lysandros","Kleon","Diodoros","Menandros","Xenon"],
        family: &["Alkmaionidai","Philaidai","Bakchiadai","Kypselidai","Eumolpidai","Pelopidai","Kerykes","Aleuadai","Battiadai","Branchidai","Asklepiadai","Lykomidai","Gephyraioi","Praxiergidai","Salaminioi","Theodoridai","Hippeis","Eteoboutadai","Peisistratidai","Kodridai","Neleidai","Medontidai","Penthilidai","Aigeidai","Boutadai","Euneidai","Amynandridai","Phytalidai","Titakidai","Kynnidai","Pamphidai","Daidalidai","Kephalidai","Erysichthonidai"],
        guild: &["Emporion","Koinon","Synedrion","Thiasos","Eranos","Symmoria","Naukleroi"],
        epithet: &["Megale","Hypsele","Akra"] },
    // 2 Phoenician / Punic
    Kit { name: "Punic", lang_family: "Semitic", env: Env::Maritime,
        on: &["Qart","Gad","Tyr","Sid","Byr","Mel","Utix","Lep","Mot","Pan","Sab","Mag","Tharr","Hadr","Bos","Kart"],
        mid: &["","a","i","o","ad","as"],
        end: &["ada","on","ath","qart","im","tis","uba","esh","ar","anto","umet","shama"],
        given: &["Hanno","Mago","Hamilcar","Bomilcar","Adherbal","Gisco","Hasdrubal","Maharbal","Bostar","Himilco","Eshmun","Zakarbaal"],
        family: &["Barcids","Magonids","Hannids","Gisconids","Melqartids","Bostarids","Tyrians","Sidonites","Eshmunids","Baalids","Gerids","Zakarbids","Hannonids","Adonids","Bodashtarids","Hannibaals","Carthalonids","Safotids","Hamilcarids","Hasdrubalids","Maharbalids","Himilcoids","Adherbalids","Saphonids","Abdmelqarts","Bomilcarids","Yadamilkids","Abibaalids","Ahirams","Mottonids","Gerastartids","Hannobaals","Bythiads","Iomilkids"],
        guild: &["Beth","Sokim","Miqdash","Mahanet","Tarsis","Kothon","Suffetim"],
        epithet: &["Rabba","Adir","the Great Harbour"] },
    // 3 Persian
    Kit { name: "Persian", lang_family: "Iranian", env: Env::Arid,
        on: &["Pasar","Ekba","Susa","Persa","Zara","Bakh","Ragha","Hyrka","Nisa","Kuru","Asha","Dara","Anshan","Gaba","Tushpa","Mithra"],
        mid: &["","a","i","an","ar","o"],
        end: &["gard","kana","dana","shahr","abad","stan","kert","vana","thra","spa","drava","mada"],
        given: &["Dariush","Kourosh","Bahram","Artashir","Kavus","Farrokh","Mithradat","Tiridat","Vologases","Narseh","Shapur","Hormizd"],
        family: &["Karen","Suren","Mihranids","Spandiyads","Varaz","Kanarang","Zarmihr","Achaemenids","Sasanids","Arsacids","Mehran","Pahlavids","Ispahbudhan","Zik","Aspahbadh","Andigan","Qaren","Dahae","Bawandids","Karinids","Sukhrids","Farrukhzads","Gondofarrids","Espahbadan","Dadbundadh","Sohaeids","Burzin","Mahgushnasp","Wahriz","Shahrbaraz","Mardanshah","Bistam","Vinduyih","Rashnu"],
        guild: &["Karwan","Anjoman","Bazaar","Rasta","Sarai","Kalantar","Ostandar"],
        epithet: &["Buzurg","the Great","Shahanshah"] },
    // 4 Norse
    Kit { name: "Norse", lang_family: "Germanic", env: Env::Cold,
        on: &["Haf","Bjor","Skag","Vester","Nor","Ulf","Thrond","Stav","Gud","Ravn","Iso","Frost","Vik","Hald","Birk","Auste"],
        mid: &["","a","e","s","ar","en"],
        end: &["vik","fjord","by","heim","stad","nes","holm","berg","dal","oy","fell","gard"],
        given: &["Bjorn","Sigurd","Ragnar","Leif","Erik","Halfdan","Ulf","Harald","Knut","Ivar","Thorvald","Gunnar"],
        family: &["Bjornsson","Sigurdarson","Ragnarsson","Ulfsson","Haraldsson","Knutsson","Ivarsson","Thorvaldsson","Gunnarsson","Eriksson","Haakonsson","Sturlung","Magnusson","Steinarsson","Vagnsson","Skjoldung","Yngling","Hlathir","Olafsson","Sveinsson","Asgrimsson","Ketilsson","Bardsson","Hroaldsson","Geirsson","Halldorsson","Egilsson","Snorrasson","Vifilsson","Ozurarson","Floksson","Arnesson","Hroarsson","Gormsson"],
        guild: &["Felag","Kaupang","Lag","Gildi","Stafnbui","Bryggja","Varda"],
        epithet: &["hinn Mikli","Storr","the Elder"] },
    // 5 Celtic
    Kit { name: "Celtic", lang_family: "Celtic", env: Env::Temperate,
        on: &["Dun","Caer","Llan","Aber","Pen","Tre","Inver","Kil","Bally","Glen","Rath","Cair","Loch","Ard","Bran","Vin"],
        mid: &["","a","y","wy","o","en"],
        end: &["dunon","briga","magos","ialon","duros","rix","vellaunum","acos","onna","ennon","bona","ritum"],
        given: &["Brennus","Vercingetor","Cathbad","Conall","Owain","Bran","Gwydion","Cunobel","Diviciac","Lugaid","Maelgwn","Tasciov"],
        family: &["Brigantes","Catuvellauni","Arverni","Aedui","Iceni","Dumnonii","Carnutes","Senones","Parisii","Veneti","Cornovii","Boii","Atrebates","Trinovantes","Ordovices","Silures","Durotriges","Coritani","Helvetii","Sequani","Lingones","Bituriges","Volcae","Pictones","Santones","Allobroges","Treveri","Nervii","Morini","Cadurci","Vocontii","Turones","Redones","Namnetes","Cenomani"],
        guild: &["Comann","Margad","Tuath","Nemeton","Cuallacht","Aonach","Ceard"],
        epithet: &["Mor","Vellaunos","the Tall"] },
    // 6 Arabic
    Kit { name: "Arab", lang_family: "Semitic", env: Env::Arid,
        on: &["Al-","Ban","Qas","Madin","Hisn","Ras","Bir","Wad","Suq","Dar","Jab","Sham","Naj","Khaf","Yath","Hadr"],
        mid: &["","a","i","u","an","ar"],
        end: &["ah","iyah","abad","iya","an","un","at","ir","ur","im","aniyah","ut"],
        given: &["Yusuf","Tariq","Khalid","Harun","Walid","Sayf","Nasir","Marwan","Qasim","Zayd","Salim","Umar"],
        family: &["al-Kindi","al-Faruqi","al-Hashimi","al-Najjar","al-Saqr","al-Rashidi","al-Bakri","al-Tujari","al-Mansuri","al-Qaysi","al-Dimashqi","al-Sahili","al-Tanukhi","al-Ghassani","al-Lakhmi","al-Azdi","al-Qurashi","al-Tamimi","al-Farabi","al-Razi","al-Tabari","al-Maqdisi","al-Andalusi","al-Misri","al-Yamani","al-Hijazi","al-Baghdadi","al-Basri","al-Kufi","al-Wasiti","al-Halabi","al-Mawsili","al-Shaybani","al-Khazraji","al-Juhani"],
        guild: &["Suq","Funduq","Tujjar","Qaysariyya","Wakala","Hisba","Sinf"],
        epithet: &["al-Kubra","al-Azim","the Radiant"] },
    // 7 Sanskritic / Indic
    Kit { name: "Indic", lang_family: "Indo-Aryan", env: Env::Tropical,
        on: &["Pata","Vara","Indra","Maha","Kasi","Ujja","Taksha","Vidi","Champa","Sura","Naga","Kanya","Praya","Dvara","Amara","Bhima"],
        mid: &["","a","i","na","ra","va"],
        end: &["pura","nagar","gram","desha","vati","stan","khanda","loka","dvipa","ashtra","kshetra","palli"],
        given: &["Chandra","Vikrama","Ashoka","Harsha","Devanand","Rajendra","Samudra","Bhima","Arjun","Kalidasa","Surya","Indra"],
        family: &["Maurya","Gupta","Chola","Pallava","Kushana","Satavahana","Vakataka","Chalukya","Pandya","Rashtrakuta","Pala","Sena","Vardhana","Kadamba","Maitraka","Pratihara","Chahamana","Hoysala","Nanda","Shunga","Kanva","Kakatiya","Yadava","Paramara","Solanki","Tomara","Chandela","Gahadavala","Sisodia","Kalachuri","Ganga","Kamboja","Licchavi","Shilahara","Chera"],
        guild: &["Shreni","Nigama","Sangha","Puga","Gana","Vanik","Mahajana"],
        epithet: &["Maha","Uttama","the Golden"] },
    // 8 Sinitic
    Kit { name: "Sinitic", lang_family: "Sinitic", env: Env::Temperate,
        on: &["Chang","Luo","Xian","Jian","Lin","Hang","Guang","Nan","Bei","Tai","Hua","Jin","Wu","Qi","Lu","Yun"],
        mid: &["","an","ing","ou","ai","ong"],
        end: &["zhou","jing","yang","an","cheng","fu","kou","shan","hai","ling","du","xi"],
        given: &["Zhao","Liang","Wen","Hui","Bo","Tai","Jun","Hong","Shen","Yuan","Kang","Ping"],
        family: &["Wang","Li","Zhang","Chen","Liu","Yang","Huang","Zhou","Wu","Sun","Zheng","Xu","Lin","He","Gao","Luo","Song","Tang","Feng","Deng","Zhao","Qin","Han","Wei","Jin","Cao","Ma","Zhu","Hu","Guo","Lu","Cai","Ye","Pan","Du","Ding","Shen","Xie","Cheng","Fan"],
        guild: &["Hang","Hui","Shanghui","Gongsuo","Bang","Zihao","Piaohao"],
        epithet: &["Da","Sheng","the Great"] },
    // 9 Slavic
    Kit { name: "Slavic", lang_family: "Slavic", env: Env::Cold,
        on: &["Nov","Bel","Veli","Krak","Smol","Cherni","Pere","Plesk","Rus","Vlad","Yaro","Sviat","Tver","Polo","Mira","Drago"],
        mid: &["","o","i","a","os","e"],
        end: &["grad","gorod","sk","ovo","ica","yn","slav","mir","polye","ov","ino","ets"],
        given: &["Vladimir","Sviatoslav","Yaroslav","Mstislav","Rostislav","Bogdan","Dragomir","Radoslav","Miroslav","Vsevolod","Boris","Gleb"],
        family: &["Rurikids","Olgovichi","Monomakhi","Drevlyane","Krivichi","Vyatichi","Polyane","Radimichi","Dregovichi","Severyane","Volhynians","Berendei","Izyaslavichi","Vseslavichi","Rostislavichi","Glebovichi","Yuryevichi","Mstislavichi","Svyatoslavichi","Vladimirovichi","Vsevolodovichi","Davydovichi","Olegovichi","Igorevichi","Yaroslavichi","Bryachislavichi","Rogvolodichi","Vasilkovichi","Romanovichi","Andreevichi","Vyacheslavichi","Tverichi","Galichane","Tysyatskie"],
        guild: &["Bratstvo","Torg","Druzhina","Artel","Sotnya","Ryad","Gostiny"],
        epithet: &["Velikiy","Slavny","the Bold"] },
    // 10 Nahuatl / Mesoamerican
    Kit { name: "Nahua", lang_family: "Nahuan", env: Env::Tropical,
        on: &["Teno","Tlate","Texco","Xochi","Tlaxca","Cholo","Teoti","Azca","Cuauh","Mixco","Tula","Mali","Coa","Itz","Tepe","Huey"],
        mid: &["","a","i","te","tla","co"],
        end: &["titlan","tlan","co","pan","tepec","apan","huacan","calco","tzinco","mila","lan","oztoc"],
        given: &["Cuauhtemoc","Moctezuma","Tlacaelel","Itzcoatl","Axayacatl","Nezahual","Chimalpopoca","Tizoc","Acamapichtli","Xiuh","Tlaloc","Quetzal"],
        family: &["Mexica","Tepaneca","Acolhua","Chichimeca","Tlaxcalteca","Totonac","Mixteca","Zapoteca","Huaxteca","Otomi","Tarasca","Mazahua","Culhua","Xochimilca","Chalca","Cuitlahuac","Mixquica","Cohuixca","Tlatelolca","Texcoca","Chimalpaneca","Matlatzinca","Coatlican","Amaquemeca","Tenanca","Iztapalapa","Coyohuaca","Azcapotzalca","Huexotzinca","Tultitlan","Tepeyacac","Coatlinchan","Acolman"],
        guild: &["Pochteca","Calpolli","Tianquiztli","Pochtlan","Oztomeca","Tealtianime"],
        epithet: &["Huey","Tlatoani","the Revered"] },
    // 11 Turkic
    Kit { name: "Turkic", lang_family: "Turkic", env: Env::Arid,
        on: &["Bal","Qara","Sar","Otra","Tash","Bukh","Sam","Talas","Aksu","Kashg","Turk","Yenis","Bes","Ulan","Altan","Bey"],
        mid: &["","a","i","y","an","ar"],
        end: &["kent","baliq","gar","tepe","shahr","oba","kol","tag","yurt","saray","bulaq","orda"],
        given: &["Bumin","Tengri","Bilge","Kultegin","Tonyukuk","Alp","Arslan","Bayan","Tarkhan","Subutai","Boru","Ertugrul"],
        family: &["Ashina","Oguz","Kipchak","Karluk","Pecheneg","Khazar","Uyghur","Bashkir","Kimek","Tatar","Bulgar","Seljuk","Qarluq","Toquz","Onoq","Turgesh","Qangli","Yagma","Qarakhanid","Avar","Cuman","Khalaj","Chigil","Yabaku","Basmyl","Sabir","Utigur","Kutrigur","Bayandur","Salur","Yiwa","Afshar","Begdili","Chepni"],
        guild: &["Orda","Kervan","Lonca","Esnaf","Ahi","Bazirgan","Tamga"],
        epithet: &["Ulug","Khan","the Wolf"] },
    // ── Minor peoples (smaller, rarer hearths) ─────────────────────────────
    // 12 Egyptian / Nilotic
    Kit { name: "Nilotic", lang_family: "Nilotic", env: Env::Arid,
        on: &["Was","Men","Aba","Per","Sais","Buto","Ineb","Kha","Ombos","Tani","Meru","Nekh","Djeba","Behdet","Iuny","Swen"],
        mid: &["","a","e","en","er","et"],
        end: &[" nefer","het","opolis","waset","meru","ankh","tep","hor","aset","ra","khet","tjenu"],
        given: &["Amenhotep","Ramesu","Sneferu","Khufu","Thutmose","Djedefre","Setna","Merenptah","Nectanebo","Psamtik","Ankhef","Nebamun"],
        family: &["Amunids","Ptahmoset","Setiu","Nubkha","Ramessids","Sematawy","Ahmoside","Khaemwaset","Panhesy","Userkaf","Meritamun","Djehutis","Bakenkhonsu","Nebseny","Paankh","Iryhor"],
        guild: &["Per-hedj","Shena","Wabet","Kap","Hut-ka","Sesh","Menat"],
        epithet: &["the Golden","Neb-Maat","of the Two Lands"] },
    // 13 Amazigh / Berber
    Kit { name: "Amazigh", lang_family: "Berber", env: Env::Arid,
        on: &["Agad","Tam","Tin","Ida","Ait"," Imi","Tafr","Ghar","Ouar","Zag","Tazr","Amel","Ifr","Tigh","Assa","Wadn"],
        mid: &["","a","i","ou","en","er"],
        end: &["ir","zgou","ratin","mlal","nadn","ghir","zzou","went","sust","dagh","fella","runt"],
        given: &["Massin","Yugurt","Aksil","Tacfin","Idir","Gaya","Firmus","Takfarinas","Amayas","Izem","Yidir","Massinissa"],
        family: &["Zenata","Sanhaja","Masmuda","Nafusa","Kutama","Awraba","Miknasa","Houara","Luwata","Gomara","Barghawata","Iznagen","Ait-Atta","Iregwaten","Ihahan","Aitmzab"],
        guild: &["Amur","Taddart","Agadir","Jmaa","Souk","Taqbilt","Amghar"],
        epithet: &["Amenokal","the Free","of the Veil"] },
    // 14 Yamato / Japonic
    Kit { name: "Yamato", lang_family: "Japonic", env: Env::Maritime,
        on: &["Yama","Kawa","Naga","Higa","Aki","Toyo","Iso","Miya","Kama","Owa","Shina","Kuni","Sato","Hira","Fuji","Naru"],
        mid: &["","a","i","no","shi","ma"],
        end: &["moto","gawa","saki","mura","oka","hara","tani","shima","dera","take","numa","bashi"],
        given: &["Takeru","Hiro","Masashi","Yorito","Kaneie","Sadao","Munenori","Tadashi","Michizane","Yoritomo","Kagemasa","Toshiie"],
        family: &["Fujiwara","Taira","Minamoto","Tachibana","Soga","Mononobe","Otomo","Abe","Hojo","Ashikaga","Hosokawa","Shimazu","Mori","Date","Uesugi","Takeda","Ii","Maeda"],
        guild: &["Za","Kumi","Toiya","Nakama","Kabu","Machishu","Kaisho"],
        epithet: &["no Kami","the Elder","Daimyo"] },
    // 15 Mongol / steppe horde
    Kit { name: "Mongol", lang_family: "Mongolic", env: Env::Cold,
        on: &["Kara","Ordu","Erden","Tsag","Bur","Khal","Nai","Kere","Merk","Oira","Bar","Onon","Selen","Tuul","Khent","Altan"],
        mid: &["","a","u","in","gh","an"],
        end: &["baatar","gol","nuur","khan","balgas","tal","uul","sum","noor","tug","yin","dai"],
        given: &["Temujin","Jochi","Chagatai","Ogedei","Subedei","Mukhali","Borte","Kublai","Tolui","Batu","Hulegu","Berke"],
        family: &["Borjigin","Kereyid","Naiman","Merkid","Tatar","Oirat","Khongirad","Jalair","Barlas","Manghud","Uriankhai","Besud","Tayichiud","Khori","Buryat","Dorbet"],
        guild: &["Ordu","Yam","Nokod","Kesig","Aimag","Khuraldai","Zud"],
        epithet: &["Khan","the Great Khan","Bagatur"] },
    // 16 Andean / Quechua
    Kit { name: "Quechua", lang_family: "Quechuan", env: Env::Tropical,
        on: &["Qusqu","Willka","Machu","Ollan","Sacsa","Pisaq","Cham","Wari","Tiwa","Cara","Chimu","Nazca","Apu","Inti","Puma","Yana"],
        mid: &["","a","i","na","pa","ri"],
        end: &["marca","pata","waman","tambo","huaca","raqay","cancha","pampa","yuq","llaqta","suyu","qucha"],
        given: &["Pachacuti","Atawallpa","Huayna","Tupac","Manco","Sinchi","Yupanqui","Wiraqucha","Amaru","Inti","Rumi","Kusi"],
        family: &["Inka","Chanka","Wanka","Chimu","Colla","Lupaca","Cañari","Chachapoya","Quilla","Ayarmaca","Wari","Tiwanaku","Moche","Nazca","Chincha","Yarovilca"],
        guild: &["Ayllu","Mita","Qhapaq","Kuraka","Chaski","Tampu","Qollqa"],
        epithet: &["Sapa","the Sun-Born","Qhapaq"] },
    // 17 Mande / Sahel
    Kit { name: "Mande", lang_family: "Mande", env: Env::Tropical,
        on: &["Wag","Nia","Kang","Segu","Dje","Timbu","Kum","Bam","Tou","Sika","Kaya","Gao","Kong","Bou","Mopt","Djen"],
        mid: &["","a","i","ou","an","ba"],
        end: &["adou","koro","dougou","ni","bara","fara","mba","sso","nkan","biri","tigi","fing"],
        given: &["Sundiata","Mansa","Sakura","Souleyman","Kankou","Fakoli","Tiramakan","Naré","Bakari","Kanku","Musa","Diata"],
        family: &["Keita","Traore","Konate","Diarra","Cisse","Kouyate","Dembele","Sissoko","Koroma","Kamara","Diakite","Sanogo","Doumbia","Fofana","Diallo","Sacko"],
        guild: &["Ton","Djeli","Marka","Wangara","Dyula","Kafu","Numu"],
        epithet: &["Mansa","the Lion","Faama"] },
];

/// Goods a people PRIZES (its cultural taste) — the luxuries/staples its markets seek.
/// Names match the goods-spec ids so the UI can show the good's emoji. Indexed by kit.
pub fn kit_desired_goods(kit: usize) -> &'static [&'static str] {
    const D: [&[&str]; 18] = [
        &["wine", "oliveoil", "wheat"],      // Roman
        &["wine", "oliveoil", "pearls"],     // Hellene
        &["dyes", "pearls", "gemstones"],    // Punic
        &["spices", "silk", "gemstones"],    // Persian
        &["furs", "amber", "whaling"],       // Norse
        &["hides", "salt", "beer"],          // Celtic
        &["frankincense", "incense", "coffee"], // Arab
        &["spices", "cotton", "gemstones"],  // Indic
        &["silk", "tea", "salt"],            // Sinitic
        &["furs", "amber", "honey"],         // Slavic
        &["cotton", "dyes", "gemstones"],    // Nahua
        &["furs", "hides", "salt"],          // Turkic
        &["wheat", "oliveoil", "incense"],   // Nilotic
        &["salt", "dyes", "hides"],          // Amazigh
        &["silk", "tea", "pearls"],          // Yamato
        &["furs", "hides", "amber"],         // Mongol
        &["cotton", "gemstones", "dyes"],    // Quechua
        &["salt", "iron", "cotton"],         // Mande
    ];
    D.get(kit).copied().unwrap_or(&[])
}

// ── Culture TRAITS ──────────────────────────────────────────────────────────
/// A cultural trait — a lasting disposition that colours a people's character and
/// (where wired) drives sim behaviour: assimilation, migration, trade, quarters.
pub struct CultureTrait {
    pub name: &'static str,
    pub emoji: &'static str,
    pub blurb: &'static str,
}

/// The fixed trait catalogue. Index order is stable (persisted implicitly via the
/// deterministic assignment below); append new traits at the END only.
pub const TRAITS: &[CultureTrait] = &[
    CultureTrait { name: "Mercantile", emoji: "💰", blurb: "born traders — wealth flows through their markets and counting-houses" },      // 0
    CultureTrait { name: "Seafaring",  emoji: "⚓", blurb: "sailors and shipwrights, at home on open water" },                              // 1
    CultureTrait { name: "Insular",    emoji: "🏝", blurb: "keep to their own — slow to mix with or adopt the ways of outsiders" },          // 2
    CultureTrait { name: "Martial",    emoji: "⚔", blurb: "a warrior tradition — quick to raise arms and prize valour" },                    // 3
    CultureTrait { name: "Devout",     emoji: "🕯", blurb: "deeply pious — faith and rite order daily life" },                               // 4
    CultureTrait { name: "Nomadic",    emoji: "🐎", blurb: "move with the seasons — few permanent walls, mobile herds and camps" },          // 5
    CultureTrait { name: "Diaspora",   emoji: "🧳", blurb: "scattered far from any homeland — keep their own quarters and wander widely" },   // 6
    CultureTrait { name: "Assimilative", emoji: "🤝", blurb: "readily absorb neighbours and are absorbed — a melting-pot people" },          // 7
    CultureTrait { name: "Clannish",   emoji: "🩸", blurb: "bound by kin and blood-loyalty above all" },                                     // 8
    CultureTrait { name: "Scholarly",  emoji: "📜", blurb: "keepers of letters, law and learning" },                                        // 9
    CultureTrait { name: "Agrarian",   emoji: "🌾", blurb: "rooted farmers of settled, tilled land" },                                       // 10
    CultureTrait { name: "Pastoral",   emoji: "🐐", blurb: "herders of the dry and open country" },                                          // 11
    CultureTrait { name: "Artisan",    emoji: "🔨", blurb: "renowned crafters — fine wares and skilled hands" },                             // 12
    CultureTrait { name: "Xenophobic", emoji: "🚫", blurb: "wary of strangers — resist foreign ways and intermarriage" },                    // 13
];

/// Deterministic traits for a culture, from its kit archetype, homeland environment,
/// travel-proneness and a stable seed. Returns 2–3 trait indices (into `TRAITS`), most
/// characteristic first. A highly mobile people gains **Diaspora + Insular** (the
/// scattered, self-contained merchant-minority profile), so real-world-style diaspora
/// cultures emerge naturally from high mobility.
pub fn kit_traits(kit: usize, mobility: f32, seed: u64) -> Vec<usize> {
    // Two archetype traits per kit (index into TRAITS).
    const ARCH: [[usize; 2]; 18] = [
        [3, 9],   // 0 Roman    — martial, scholarly (law)
        [1, 9],   // 1 Hellene  — seafaring, scholarly
        [0, 1],   // 2 Punic    — mercantile, seafaring
        [9, 4],   // 3 Persian  — scholarly, devout
        [3, 1],   // 4 Norse    — martial, seafaring
        [8, 3],   // 5 Celtic   — clannish, martial
        [0, 4],   // 6 Arab     — mercantile, devout
        [4, 12],  // 7 Indic    — devout, artisan
        [9, 12],  // 8 Sinitic  — scholarly, artisan
        [10, 8],  // 9 Slavic   — agrarian, clannish
        [4, 12],  // 10 Nahua   — devout, artisan
        [5, 11],  // 11 Turkic  — nomadic, pastoral
        [11, 5],  // 12 Nilotic — pastoral, nomadic
        [11, 0],  // 13 Amazigh — pastoral, mercantile (caravans)
        [12, 4],  // 14 Yamato  — artisan, devout
        [5, 3],   // 15 Mongol  — nomadic, martial
        [10, 12], // 16 Quechua — agrarian, artisan
        [0, 11],  // 17 Mande   — mercantile, pastoral
    ];
    let mut out: Vec<usize> = Vec::new();
    let mut push = |out: &mut Vec<usize>, id: usize| { if !out.contains(&id) { out.push(id); } };
    if let Some(a) = ARCH.get(kit) { push(&mut out, a[0]); push(&mut out, a[1]); }
    // Travel-proneness → the diaspora/nomad axis.
    if mobility >= 0.68 { push(&mut out, 6); push(&mut out, 2); }      // Diaspora + Insular
    else if mobility >= 0.5 { push(&mut out, 5); }                     // Nomadic
    else if mobility <= 0.22 { push(&mut out, 10); }                   // very rooted → Agrarian
    // A seeded flavour trait for variety when there's still room.
    if out.len() < 3 {
        let flavour = [7usize, 13, 12, 8, 9, 0][(seed % 6) as usize];
        push(&mut out, flavour);
    }
    out.truncate(3);
    out
}

/// Does a culture with these trait indices strongly RESIST assimilation? Insular,
/// Xenophobic and Diaspora peoples keep to themselves (their minorities persist as
/// quarters instead of dissolving); Assimilative peoples do the opposite.
pub fn traits_resist_assimilation(traits: &[usize]) -> f32 {
    let mut r = 1.0f32;
    if traits.contains(&2) { r *= 0.45; }  // Insular
    if traits.contains(&13) { r *= 0.5; }  // Xenophobic
    if traits.contains(&6) { r *= 0.4; }   // Diaspora
    if traits.contains(&7) { r *= 1.6; }   // Assimilative (dissolves faster)
    r.clamp(0.15, 1.8)
}

/// Candidate kit indices for a hearth's environment (the kit's `env` matching the
/// hearth cell biases selection; any kit can still appear via the hash tiebreak).
fn env_of_cell(buf: &WorldBuffer, idx: usize) -> Env {
    let k = buf.koppen[idx];
    let t = buf.temperature[idx];
    if matches!(k, 4 | 5 | 6 | 7) { return Env::Arid; }
    if t <= 2.0 { return Env::Cold; }
    if matches!(k, 1 | 2 | 3) || t >= 24.0 { return Env::Tropical; } // Af/Am/Aw tropical
    if buf.distance_to_ocean[idx] < 0.03 { return Env::Maritime; }
    Env::Temperate
}

// ── The computed culture map ───────────────────────────────────────────────────
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Hearth {
    pub x: f32,
    pub y: f32,
    pub kit: u8,
    pub mut_seed: u64,
    pub people: String, // region / people label, e.g. "Vexillia"
    pub color: [u8; 3],
    /// Language family this people belongs to (from its kit) — relatedness drives how
    /// readily its minorities assimilate elsewhere. `#[serde(default)]` → old saves fill "".
    #[serde(default)] pub family: String,
    /// A STATIC origin card written at worldgen: where this people arose (homeland /
    /// biome), their language family, and their temperament (rooted vs migration-prone).
    /// Fixed for the life of the world. `#[serde(default)]` → old saves show "".
    #[serde(default)] pub origin: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CultureMap {
    pub fp: u64,            // fingerprint (seed ⊕ dims ⊕ hearths)
    pub world_w: u32,
    pub world_h: u32,
    pub cw: u32,            // coarse raster width
    pub ch: u32,            // coarse raster height
    pub ids: Vec<u8>,       // hearth index per coarse cell (255 = unassigned)
    pub hearths: Vec<Hearth>,
}

impl CultureMap {
    #[inline]
    fn coarse_of(&self, x: u32, y: u32) -> usize {
        let cx = (x as u64 * self.cw as u64 / self.world_w.max(1) as u64).min(self.cw as u64 - 1) as u32;
        let cy = (y as u64 * self.ch as u64 / self.world_h.max(1) as u64).min(self.ch as u64 - 1) as u32;
        (cy * self.cw + cx) as usize
    }
    /// The hearth governing cell `(x,y)` — the coarse raster if assigned, else the
    /// nearest hearth by wrapped distance (so islands still get a culture).
    pub fn hearth_at(&self, x: u32, y: u32) -> Option<&Hearth> {
        if self.hearths.is_empty() { return None; }
        let id = self.ids.get(self.coarse_of(x, y)).copied().unwrap_or(255);
        if (id as usize) < self.hearths.len() {
            return self.hearths.get(id as usize);
        }
        // Fallback: nearest hearth by Euclidean (X wraps).
        let mut best = (usize::MAX, f32::MAX);
        for (i, hh) in self.hearths.iter().enumerate() {
            let mut dx = (hh.x - x as f32).abs();
            let ww = self.world_w as f32;
            if ww > 1.0 { dx = dx.min(ww - dx); }
            let dy = hh.y - y as f32;
            let d = dx * dx + dy * dy;
            if d < best.1 { best = (i, d); }
        }
        self.hearths.get(best.0)
    }
    pub fn kit_at(&self, x: u32, y: u32) -> Option<(usize, u64)> {
        self.hearth_at(x, y).map(|h| (h.kit as usize, h.mut_seed))
    }
}

// ── Process-global active map ──────────────────────────────────────────────────
static ACTIVE: RwLock<Option<Arc<CultureMap>>> = RwLock::new(None);
static ACTIVE_FP: AtomicU64 = AtomicU64::new(0);

pub fn set_active(map: CultureMap) {
    ACTIVE_FP.store(map.fp, Ordering::Relaxed);
    *ACTIVE.write().unwrap() = Some(Arc::new(map));
}
pub fn clear_active() {
    ACTIVE_FP.store(0, Ordering::Relaxed);
    *ACTIVE.write().unwrap() = None;
}
pub fn active() -> Option<Arc<CultureMap>> {
    ACTIVE.read().unwrap().clone()
}

/// Make sure the global map matches the world in `conn` (cheap: compares a stored
/// fingerprint against the cached one). Reads + parses the map only on a miss.
pub fn ensure_active(conn: &rusqlite::Connection) {
    let fp = crate::db::metadata::get_meta(conn, "cultures_fp")
        .ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    if fp == ACTIVE_FP.load(Ordering::Relaxed) {
        if fp == 0 { clear_active(); }
        return;
    }
    if fp == 0 {
        clear_active();
        return;
    }
    if let Ok(Some(json)) = crate::db::metadata::get_meta(conn, "cultures") {
        if let Ok(map) = serde_json::from_str::<CultureMap>(&json) {
            set_active(map);
            return;
        }
    }
    clear_active();
}

/// Persist a freshly-computed map into world metadata and make it active.
pub fn store_and_activate(conn: &rusqlite::Connection, map: CultureMap) -> rusqlite::Result<()> {
    let fp = map.fp;
    let json = serde_json::to_string(&map).unwrap_or_default();
    crate::db::metadata::set_meta(conn, "cultures", &json)?;
    crate::db::metadata::set_meta(conn, "cultures_fp", &fp.to_string())?;
    set_active(map);
    Ok(())
}

// ── Computing the map ───────────────────────────────────────────────────────────
/// A pleasant, well-spread palette for culture territories.
const CULTURE_PALETTE: &[[u8; 3]] = &[
    [201, 122, 84], [86, 142, 196], [122, 184, 110], [206, 168, 90], [150, 120, 196],
    [110, 184, 176], [196, 110, 140], [120, 156, 96], [176, 140, 92], [96, 132, 176],
    [186, 96, 96], [128, 176, 120], [150, 130, 110], [110, 150, 150],
];

/// Compute the organic culture map from terrain/climate. `seed` is the world seed.
pub fn compute_culture_map(buf: &WorldBuffer, seed: u64, desired: Option<usize>) -> CultureMap {
    let w = buf.width;
    let h = buf.height;
    // Coarse raster: ~280 cells across, proportional height.
    let cw = w.min(280).max(40);
    let ch = (cw * h / w.max(1)).max(20);
    let ccount = (cw * ch) as usize;

    // Build coarse land + cost grid (sea = impassable).
    let mut land = vec![false; ccount];
    let mut cost = vec![f32::INFINITY; ccount]; // step cost to ENTER this coarse cell
    let mut land_cells = 0u32;
    for cy in 0..ch {
        for cx in 0..cw {
            // Sample the full-res cell at the coarse cell centre.
            let fx = ((cx as u64 * w as u64 + w as u64 / 2) / cw as u64).min(w as u64 - 1) as u32;
            let fy = ((cy as u64 * h as u64 + h as u64 / 2) / ch as u64).min(h as u64 - 1) as u32;
            let i = buf.idx(fx, fy);
            let ci = (cy * cw + cx) as usize;
            if buf.terrain[i] == 1 {
                land[ci] = true;
                land_cells += 1;
                // Cheap across open ground; mountains and deserts are dear barriers.
                let mut c = 1.0f32;
                if buf.elevation[i] > 0.45 { c += 6.0 * ((buf.elevation[i] - 0.45) / 0.3).min(1.0); }
                if matches!(buf.koppen[i], 4 | 5) { c += 3.0; }       // hot/cold desert
                if matches!(buf.koppen[i], 6 | 7) { c += 1.2; }       // steppe
                cost[ci] = c;
            }
        }
    }

    // How many hearths (initial cultures): the user's chosen count if given (clamped to a
    // sane range for the map's land area), else auto-scaled with land area.
    let auto = ((land_cells as f32).sqrt() / 9.0).round().clamp(5.0, 14.0) as usize;
    let hard_max = ((land_cells as f32).sqrt() / 4.0).round().max(6.0) as usize; // enough room to space seeds
    let k = match desired {
        Some(d) if d >= 1 => d.clamp(1, hard_max.max(1)),
        _ => auto,
    };

    // Seed candidate cells: land, mild, low — ranked by a habitability-ish score
    // plus seeded noise, then greedily chosen with a minimum spacing.
    let mut cand: Vec<(usize, f32)> = Vec::new();
    for ci in 0..ccount {
        if !land[ci] { continue; }
        let cx = (ci as u32) % cw;
        let cy = (ci as u32) / cw;
        let fx = ((cx as u64 * w as u64 + w as u64 / 2) / cw as u64).min(w as u64 - 1) as u32;
        let fy = ((cy as u64 * h as u64 + h as u64 / 2) / ch as u64).min(h as u64 - 1) as u32;
        let i = buf.idx(fx, fy);
        let t = buf.temperature[i];
        let mild = 1.0 - ((t - 16.0).abs() / 28.0).clamp(0.0, 1.0);
        let low = 1.0 - (buf.elevation[i] / 0.6).clamp(0.0, 1.0);
        let score = 0.55 * mild + 0.25 * low + 0.20 * h01(seed ^ ci as u64 ^ 0x5EED);
        cand.push((ci, score));
    }
    cand.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let min_sep = ((cw as f32) / (k as f32)).max(3.0);
    let mut seeds: Vec<usize> = Vec::new();
    for (ci, _) in &cand {
        if seeds.len() >= k { break; }
        let cx = (*ci as u32 % cw) as i32;
        let cy = (*ci as u32 / cw) as i32;
        let far = seeds.iter().all(|&s| {
            let sx = (s as u32 % cw) as i32;
            let sy = (s as u32 / cw) as i32;
            let mut dx = (cx - sx).abs();
            if dx > cw as i32 / 2 { dx = cw as i32 - dx; }
            let dy = cy - sy;
            ((dx * dx + dy * dy) as f32).sqrt() >= min_sep
        });
        if far { seeds.push(*ci); }
    }
    if seeds.is_empty() {
        if let Some((ci, _)) = cand.first() { seeds.push(*ci); }
    }

    // Assign each hearth a kit (env-biased) and a per-world mutation + people name.
    let mut hearths: Vec<Hearth> = Vec::new();
    for (hi, &s) in seeds.iter().enumerate() {
        let cx = s as u32 % cw;
        let cy = s as u32 / cw;
        let fx = ((cx as u64 * w as u64 + w as u64 / 2) / cw as u64).min(w as u64 - 1) as u32;
        let fy = ((cy as u64 * h as u64 + h as u64 / 2) / ch as u64).min(h as u64 - 1) as u32;
        let i = buf.idx(fx, fy);
        let env = env_of_cell(buf, i);
        let kit = pick_kit(env, seed, hi, &hearths);
        let mut_seed = hash64(seed ^ (hi as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ 0xC0117);
        let people = gen_people_name(kit, mut_seed, fx, fy);
        let color = CULTURE_PALETTE[hi % CULTURE_PALETTE.len()];
        let family = KITS[kit.min(KITS.len() - 1)].lang_family.to_string();
        let origin = origin_card(&people, kit, env);
        hearths.push(Hearth { x: fx as f32, y: fy as f32, kit: kit as u8, mut_seed, people, color, family, origin });
    }

    // Multi-source least-cost flood-fill over the coarse land grid (Dijkstra; X wraps).
    let ids = flood_fill(cw, ch, &land, &cost, &seeds);

    // Fingerprint.
    let mut fp = hash64(seed ^ ((w as u64) << 32) ^ h as u64);
    for hh in &hearths {
        fp = hash64(fp ^ (hh.x as u64) ^ ((hh.y as u64) << 16) ^ ((hh.kit as u64) << 32) ^ hh.mut_seed);
    }

    CultureMap { fp, world_w: w, world_h: h, cw, ch, ids, hearths }
}

/// Choose a kit for `env`, biased toward kits native to that environment and
/// avoiding repeating a kit already used when an alternative exists.
fn pick_kit(env: Env, seed: u64, hi: usize, used: &[Hearth]) -> usize {
    let mut weights: Vec<(usize, f32)> = (0..KITS.len())
        .map(|k| {
            let mut wgt = if KITS[k].env == env { 3.0 } else { 1.0 };
            if used.iter().any(|h| h.kit as usize == k) { wgt *= 0.25; } // discourage dupes
            (k, wgt)
        })
        .collect();
    let total: f32 = weights.iter().map(|(_, w)| *w).sum();
    let mut r = h01(seed ^ (hi as u64).wrapping_mul(0x2545F4914F6CDD1D) ^ 0x4B17u64) * total;
    for (k, wgt) in weights.drain(..) {
        if r < wgt { return k; }
        r -= wgt;
    }
    0
}

/// Multi-source Dijkstra over the coarse grid. Returns hearth index per coarse
/// cell (255 = unreachable / sea). X wraps, Y clamps.
fn flood_fill(cw: u32, ch: u32, land: &[bool], cost: &[f32], seeds: &[usize]) -> Vec<u8> {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;
    #[derive(PartialEq)]
    struct Node { d: f32, ci: u32, src: u8 }
    impl Eq for Node {}
    impl PartialOrd for Node { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }
    impl Ord for Node {
        fn cmp(&self, o: &Self) -> Ordering {
            // min-heap by distance
            o.d.partial_cmp(&self.d).unwrap_or(Ordering::Equal)
        }
    }
    let n = (cw * ch) as usize;
    let mut dist = vec![f32::INFINITY; n];
    let mut owner = vec![255u8; n];
    let mut heap = BinaryHeap::new();
    for (si, &ci) in seeds.iter().enumerate() {
        if ci < n && land[ci] {
            dist[ci] = 0.0;
            owner[ci] = si as u8;
            heap.push(Node { d: 0.0, ci: ci as u32, src: si as u8 });
        }
    }
    while let Some(Node { d, ci, src }) = heap.pop() {
        if d > dist[ci as usize] { continue; }
        let cx = (ci % cw) as i32;
        let cy = (ci / cw) as i32;
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
            let mut nx = cx + dx;
            if nx < 0 { nx = cw as i32 - 1; } else if nx >= cw as i32 { nx = 0; } // wrap X
            let ny = cy + dy;
            if ny < 0 || ny >= ch as i32 { continue; }                            // clamp Y
            let ni = (ny as u32 * cw + nx as u32) as usize;
            if !land[ni] { continue; }
            let nd = d + cost[ni];
            if nd < dist[ni] {
                dist[ni] = nd;
                owner[ni] = src;
                heap.push(Node { d: nd, ci: ni as u32, src });
            }
        }
    }
    owner
}

// ── Name generation primitives (kit-aware, mutation-aware) ──────────────────────
#[inline]
fn pick<'a>(bank: &[&'a str], h: u64) -> &'a str {
    if bank.is_empty() { return ""; }
    bank[(h % bank.len() as u64) as usize]
}

/// A gentle, world-consistent phoneme shift so a kit reads a little differently
/// each world (e.g. all 'k'→'c', or 'os'→'us' endings). Chosen by `mut_seed`.
pub fn mutate(s: &str, mut_seed: u64) -> String {
    if mut_seed == 0 { return s.to_string(); }
    match mut_seed % 6 {
        0 => s.to_string(),
        1 => s.replace('k', "c"),
        2 => s.replace("os", "us").replace("on", "un"),
        3 => s.replace('y', "i"),
        4 => s.replace("th", "dh").replace('q', "k"),
        _ => s.replace("ia", "ya").replace("ae", "e"),
    }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Place name for a cell under `kit` (mutation `ms`), deterministic from position.
pub fn place_name(kit: usize, ms: u64, x: u32, y: u32) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let base = hash64((x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663));
    let raw = format!("{}{}{}", pick(k.on, base), pick(k.mid, base >> 8), pick(k.end, base >> 16));
    let raw: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    cap_first(&mutate(&raw, ms))
}

/// Hydronym (water-word) suiting a kit's environment, so a river reads with a
/// culturally-flavoured "river" element — Persian *rud*, Arabic *nahr*, Sanskrit
/// *nadi*, Greek *potamos*, Celtic *avon*, Norse *elv* … Kept per-`Env` (five
/// small banks) rather than per-kit to stay compact while still fitting the land.
fn hydronym(env: Env, h: u64) -> &'static str {
    let bank: &[&str] = match env {
        Env::Cold => &["elv", "aan", "foss", "bekk", "strom", "vatn", "aa"],
        Env::Arid => &["rud", "nahr", "joi", "wadi", "darya", "ab", "chai"],
        Env::Tropical => &["nadi", "ganga", "kali", "batang", "apo", "krung", "song"],
        Env::Maritime => &["potamos", "rhoas", "naros", "aigos", "roas", "poros"],
        Env::Temperate => &["water", "avon", "dwr", "flumen", "isca", "brook", "usk"],
    };
    pick(bank, h)
}

/// A culture-styled RIVER name built from the kit's on/mid/end stem plus (often)
/// an environment-appropriate hydronym element. `seed` fully drives the draw, so
/// a caller can re-roll with a different seed to break a duplicate. Arid cultures
/// take the water-word as a prefix ("Nahr Aldan"); the rest as a suffix
/// ("Aldan Isca"). The stem alone spans |on|×|mid|×|end| combinations (≈1000+ per
/// kit), and the hydronym multiplies that further, so unique names are plentiful.
pub fn river_name(kit: usize, ms: u64, seed: u64) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let base = hash64(seed ^ 0x0D15_EA5E_9E37_79B1);
    let stem_raw = format!("{}{}{}", pick(k.on, base), pick(k.mid, base >> 8), pick(k.end, base >> 16));
    let stem_raw: String = stem_raw.split_whitespace().collect::<Vec<_>>().join("");
    let stem = cap_first(&mutate(&stem_raw, ms));
    // ~70% of rivers carry a water-word; the rest stay bare (still unique stems).
    if (base >> 40) % 10 < 7 {
        let word = cap_first(&mutate(hydronym(k.env, base >> 24), ms));
        if k.env == Env::Arid {
            format!("{} {}", word, stem) // "Nahr Aldan"
        } else {
            format!("{} {}", stem, word) // "Aldan Isca"
        }
    } else {
        stem
    }
}

/// Deterministic per-people MOBILITY (0..1) from the people label — MUST match
/// `tick::CampaignSim::culture_mobility` so the origin card's temperament agrees with
/// how the people actually spreads in the sim. ~20% are travel-prone (≥0.7).
pub fn people_mobility(name: &str) -> f32 {
    if name.is_empty() || name == "—" { return 0.2; }
    let mut h = 0xcbf29ce484222325u64;
    for b in name.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    let r = (h % 1000) as f32 / 1000.0;
    if r > 0.80 { 0.7 + (r - 0.80) / 0.20 * 0.30 } else { 0.1 + r / 0.80 * 0.4 }
}

/// The homeland biome phrase for a hearth's environment (used in the origin card).
fn homeland_phrase(env: Env) -> &'static str {
    match env {
        Env::Cold => "the cold northern coasts and pine forests",
        Env::Arid => "the arid steppes and desert margins",
        Env::Tropical => "the hot, wet tropics",
        Env::Maritime => "a sheltered, island-strewn seaboard",
        Env::Temperate => "a temperate, river-fed heartland",
    }
}

/// A STATIC origin card for a people: where they arose, their language family, and
/// their temperament (rooted vs migration-prone). Deterministic; written once at
/// worldgen and never rewritten (the LIVE present-spread is shown separately in the UI).
pub fn origin_card(people: &str, kit: usize, env: Env) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let mob = people_mobility(people);
    let temperament = if mob >= 0.7 {
        "A restless, seafaring people: their merchant quarters spread far along the trade roads, so kin of theirs turn up settled in distant ports."
    } else if mob >= 0.45 {
        "A people of moderate wanderlust — traders and settlers who reach neighbouring lands but keep a firm homeland."
    } else {
        "A rooted, sedentary people, slow to mingle far beyond their own homeland."
    };
    let article = if matches!(k.lang_family.chars().next(), Some('A' | 'E' | 'I' | 'O' | 'U')) { "an" } else { "a" };
    format!(
        "{people} — {article} {family} people who speak a {tongue}-styled tongue — arose on {home}. {temperament}",
        people = people, article = article, family = k.lang_family, tongue = k.name,
        home = homeland_phrase(env), temperament = temperament,
    )
}

/// The kit index of a live people (by its label), from the active worldgen map, if any.
pub fn kit_of_people(name: &str) -> Option<usize> {
    active().and_then(|m| m.hearths.iter().find(|h| h.people == name).map(|h| h.kit as usize))
}

/// A kit's native environment as a small index (0 Temperate · 1 Cold · 2 Arid ·
/// 3 Tropical · 4 Maritime) — the "appearance group" a people looks like (skin/dress
/// track climate). Drives the ethnic-appearance affinity in assimilation.
pub fn kit_env_index(kit: usize) -> u8 {
    match KITS[kit.min(KITS.len() - 1)].env {
        Env::Temperate => 0, Env::Cold => 1, Env::Arid => 2, Env::Tropical => 3, Env::Maritime => 4,
    }
}

/// The appearance-group index of a live people (by label), via its kit, if known.
pub fn people_env(name: &str) -> Option<u8> {
    kit_of_people(name).map(kit_env_index)
}

/// The hearth colour of a live people (by its label), from the active map, if any.
pub fn color_of_people(name: &str) -> Option<[u8; 3]> {
    active().and_then(|m| m.hearths.iter().find(|h| h.people == name).map(|h| h.color))
}

/// Cultures 2.0 · synthesize a CREOLE name from two parent kits' word-banks: the onset
/// of parent A + a middle/ending of parent B (e.g. Roman "Nov" + Norse "vik" → "Novvik").
/// Deterministic in `seed` so a caller can re-roll to avoid a duplicate.
pub fn blend_name(kit_a: usize, kit_b: usize, seed: u64) -> String {
    let a = &KITS[kit_a.min(KITS.len() - 1)];
    let b = &KITS[kit_b.min(KITS.len() - 1)];
    let base = hash64(seed ^ 0x9E37_79B9_7F4A_7C15);
    let raw = format!("{}{}{}", pick(a.on, base), pick(b.mid, base >> 8), pick(b.end, base >> 16));
    let raw: String = raw.split_whitespace().collect::<Vec<_>>().join("");
    cap_first(&raw)
}

/// A region / people label for a hearth (a grander homeland name).
pub fn gen_people_name(kit: usize, ms: u64, x: u32, y: u32) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let base = hash64(0x9EE9u64.wrapping_add((x as u64) << 20 ^ y as u64));
    let raw = format!("{}{}", pick(k.on, base), pick(k.end, base >> 12));
    cap_first(&mutate(&raw, ms))
}

pub fn family_name(kit: usize, ms: u64, salt: u64, x: u32, y: u32) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let hh = hash64(0xFA ^ salt.wrapping_mul(0x9E3779B97F4A7C15)
        ^ (x as u64).wrapping_mul(40503) ^ (y as u64).wrapping_mul(20507));
    mutate(pick(k.family, hh), ms)
}

pub fn head_name(kit: usize, ms: u64, family: &str, salt: u64, x: u32, y: u32) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let hh = hash64(0x4EAD ^ salt.wrapping_mul(0xBF58476D1CE4E5B9)
        ^ (x as u64).wrapping_mul(73856093) ^ (y as u64).wrapping_mul(19349663));
    format!("{} {}", mutate(pick(k.given, hh), ms), family)
}

/// Culture-styled guild name: e.g. "Collegium of Aquentia (wine)".
pub fn guild_name(kit: usize, ms: u64, city: &str, specialty: Option<&str>, salt: u64) -> String {
    let k = &KITS[kit.min(KITS.len() - 1)];
    let hh = hash64(0x6111 ^ salt.wrapping_mul(0x2545F4914F6CDD1D));
    let word = mutate(pick(k.guild, hh), ms);
    // Two name patterns by hash so guild names aren't all "{word} of {city}".
    let prefix = (hh >> 19) & 1 == 1;
    match specialty {
        Some(sp) if !sp.is_empty() => if prefix {
            format!("{} {} ({})", city, word, sp)
        } else {
            format!("{} of {} ({})", word, city, sp)
        },
        _ => if prefix { format!("{} {}", city, word) } else { format!("{} of {}", word, city) },
    }
}

pub fn place_epithet(kit: usize, ms: u64, x: u32, y: u32, tier: u8) -> String {
    let name = place_name(kit, ms, x, y);
    if tier == 0 { return name; }
    let k = &KITS[kit.min(KITS.len() - 1)];
    let h2 = hash64((x as u64) ^ (y as u64).wrapping_mul(0xABCD));
    format!("{} {}", name, pick(k.epithet, h2))
}

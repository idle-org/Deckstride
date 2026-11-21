//! Allocation-free interface catalogs; worker diagnostics remain in their original language.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    En,
    Fr,
    Es,
    Ja,
    ZhCn,
    De,
    PtBr,
}
impl Language {
    pub const ALL: [Self; 8] = [
        Self::System,
        Self::En,
        Self::Fr,
        Self::Es,
        Self::Ja,
        Self::ZhCn,
        Self::De,
        Self::PtBr,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "System language",
            Self::En => "English",
            Self::Fr => "Français",
            Self::Es => "Español",
            Self::Ja => "Japanese",
            Self::ZhCn => "Chinese (Simplified)",
            Self::De => "Deutsch",
            Self::PtBr => "Português (Brasil)",
        }
    }
    pub fn french(self) -> bool {
        self.resolved() == ResolvedLanguage::Fr
    }
    fn resolved(self) -> ResolvedLanguage {
        match self {
            Self::System => system_language(),
            Self::En => ResolvedLanguage::En,
            Self::Fr => ResolvedLanguage::Fr,
            Self::Es => ResolvedLanguage::Es,
            Self::Ja => ResolvedLanguage::Ja,
            Self::ZhCn => ResolvedLanguage::ZhCn,
            Self::De => ResolvedLanguage::De,
            Self::PtBr => ResolvedLanguage::PtBr,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResolvedLanguage {
    En,
    Fr,
    Es,
    Ja,
    ZhCn,
    De,
    PtBr,
}

fn language_id() -> egui::Id {
    egui::Id::new("pdf-presenter-language")
}

fn resolve_system_locale(locale: Option<&str>) -> ResolvedLanguage {
    let locale = locale
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
        .replace('_', "-")
        .to_ascii_lowercase();
    let parts: Vec<_> = locale.split('-').collect();
    match parts.first().copied().unwrap_or("") {
        "fr" => ResolvedLanguage::Fr,
        "es" => ResolvedLanguage::Es,
        "ja" => ResolvedLanguage::Ja,
        "de" => ResolvedLanguage::De,
        "pt" if parts.contains(&"br") => ResolvedLanguage::PtBr,
        // Do not silently substitute Simplified Chinese for Traditional Chinese.
        "zh" if !parts
            .iter()
            .any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo")) =>
        {
            ResolvedLanguage::ZhCn
        }
        _ => ResolvedLanguage::En,
    }
}

fn system_language() -> ResolvedLanguage {
    static LANGUAGE: std::sync::OnceLock<ResolvedLanguage> = std::sync::OnceLock::new();
    *LANGUAGE.get_or_init(|| resolve_system_locale(sys_locale::get_locale().as_deref()))
}

pub fn apply(ctx: &egui::Context, language: &Language) {
    let resolved = language.resolved();
    let previous = ctx.data(|data| data.get_temp::<ResolvedLanguage>(language_id()));
    if previous == Some(resolved) {
        return;
    }
    if matches!(resolved, ResolvedLanguage::Ja | ResolvedLanguage::ZhCn) {
        ctx.set_fonts(cjk_fonts(resolved));
    } else if previous
        .is_some_and(|language| matches!(language, ResolvedLanguage::Ja | ResolvedLanguage::ZhCn))
    {
        ctx.set_fonts(egui::FontDefinitions::default());
    }
    ctx.data_mut(|data| data.insert_temp(language_id(), resolved));
}

fn cjk_fonts(language: ResolvedLanguage) -> egui::FontDefinitions {
    use egui::{FontData, FontFamily};
    let mut fonts = egui::FontDefinitions::default();
    let mut data = FontData::from_static(include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc"));
    data.index = if language == ResolvedLanguage::ZhCn {
        2
    } else {
        0
    };
    fonts.font_data.insert("noto-cjk".into(), data.into());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .expect("default font family")
            .insert(0, "noto-cjk".into());
    }
    fonts
}

pub fn tr<'a>(ctx: &egui::Context, text: &'a str) -> &'a str {
    match ctx
        .data(|data| data.get_temp::<ResolvedLanguage>(language_id()))
        .unwrap_or(ResolvedLanguage::En)
    {
        ResolvedLanguage::En => text,
        ResolvedLanguage::Fr => fr(text),
        ResolvedLanguage::Es => es(text),
        ResolvedLanguage::Ja => ja(text),
        ResolvedLanguage::ZhCn => zh_cn(text),
        ResolvedLanguage::De => de(text),
        ResolvedLanguage::PtBr => pt_br(text),
    }
}

pub fn display_revert_countdown(ctx: &egui::Context, seconds: u64) -> String {
    match ctx
        .data(|data| data.get_temp::<ResolvedLanguage>(language_id()))
        .unwrap_or(ResolvedLanguage::En)
    {
        ResolvedLanguage::Fr => {
            format!("Retour automatique dans {seconds} s. Échap pour annuler.")
        }
        ResolvedLanguage::Es => {
            format!("Se revertirá automáticamente en {seconds} s. Escape para revertir.")
        }
        ResolvedLanguage::Ja => {
            format!("{seconds}秒後に自動的に元に戻ります。Escapeキーで戻します。")
        }
        ResolvedLanguage::ZhCn => {
            format!("将在 {seconds} 秒后自动还原。按 Escape 立即还原。")
        }
        ResolvedLanguage::De => {
            format!("Automatische Rückkehr in {seconds} s. Escape zum Zurücksetzen.")
        }
        ResolvedLanguage::PtBr => {
            format!("Reversão automática em {seconds} s. Escape para reverter.")
        }
        ResolvedLanguage::En => {
            format!("Automatically reverts in {seconds} seconds. Escape to revert.")
        }
    }
}

fn fr(text: &str) -> &str {
    match text {
        "Disk cache" => "Cache disque",
        "Maximum total size (MiB)" => "Taille totale maximale (Mio)",
        "Expire after (days since rendering)" => "Expiration (jours depuis le rendu)",
        "Oldest files are removed first. Active decks are protected and may exceed the limit. Automatic cleanup runs once at startup, after loading and preparation finish." => {
            "Les fichiers les plus anciens sont supprimés en premier. Les présentations actives sont protégées et peuvent dépasser la limite. Nettoyage automatique une fois au démarrage, après le chargement et la préparation."
        }
        "Expunge unused cache now" => "Purger le cache inutilisé maintenant",
        "Keep fitted page area when zooming" => {
            "Conserver la zone ajustée de la page pendant le zoom"
        }
        "By default, zoom expands the page to fill the screen without stretching. Enable this option to keep the original page-shaped area." => {
            "Par défaut, le zoom agrandit la page pour remplir l’écran sans déformation. Activez cette option pour conserver la zone initiale de la page."
        }
        "Enable page zoom" => "Activer le zoom des diapositives",
        "Page zoom" => "Zoom de la diapositive",
        "Reset zoom" => "Réinitialiser le zoom",
        "Disable zoom" => "Désactiver le zoom",
        "Keep zoom enabled" => "Garder le zoom activé",
        "Zoom performance" => "Performances du zoom",
        "Scroll to zoom · drag to pan · double-click to fit width / reset" => {
            "Molette pour zoomer · glisser pour déplacer · double-clic pour ajuster à la largeur / réinitialiser"
        }
        "Use the current slide preview or a Slides screen. Changing slides resets zoom." => {
            "Utilisez l’aperçu actuel ou un écran de présentation. Changer de diapositive réinitialise le zoom."
        }
        "Zoom reuses buffered images: no extra PDF rendering or image memory. Enlarged slides may look softer. Software drawing can slow down on large or multiple displays." => {
            "Le zoom réutilise les images en mémoire, sans rendu PDF ni mémoire d’image supplémentaires. Les diapositives agrandies peuvent être moins nettes. Le dessin logiciel peut ralentir sur des écrans grands ou multiples."
        }
        "Zoomed frames are taking over 33 ms to draw. Panning may feel slow on this display. Reset or disable zoom to return to normal viewing." => {
            "Les images zoomées prennent plus de 33 ms à dessiner. Le déplacement peut sembler lent sur cet écran. Réinitialisez ou désactivez le zoom pour revenir à l’affichage normal."
        }
        "Cancel" => "Annuler",
        "Preview (first 3 slides)" => "Aperçu (3 premières diapositives)",
        "Import notes…" => "Importer des notes…",
        "Import notes" => "Importer des notes",
        "Processing notes…" => "Traitement des notes…",
        "Slides to update" => "Diapositives à modifier",
        "Sections replace notes starting at slide 1. Remaining slides keep their notes." => {
            "Les sections remplacent les notes à partir de la diapositive 1. Les suivantes conservent leurs notes."
        }
        "Update existing notes file" => "Mettre à jour le fichier de notes existant",
        "Create notes file" => "Créer le fichier de notes",
        "Empty note" => "Note vide",
        "Import and save notes" => "Importer et enregistrer les notes",
        "File chooser already open" => "Sélecteur de fichiers déjà ouvert",
        "Page background" => "Fond de page",
        "Fit padding color" => "Couleur des marges",
        "Page background fills transparency; fit padding fills space around the slide." => {
            "Le fond de page remplit les zones transparentes ; les marges remplissent l’espace autour de la diapositive."
        }
        "Appearance" => "Apparence",
        "Preferences" => "Préférences",
        "Preferences…" => "Préférences…",
        "Quick settings" => "Réglages rapides",
        "Memory and cache" => "Mémoire et cache",
        "Storage and preparation" => "Stockage et préparation",
        "Application executable" => "Exécutable de l’application",
        "This is the running portable executable. Use its containing folder to find or remove the application." => {
            "Il s’agit de l’exécutable portable en cours d’utilisation. Son dossier permet de retrouver ou supprimer l’application."
        }
        "Storage locations" => "Emplacements de stockage",
        "Preparing slide…" => "Préparation de la diapositive…",
        "Slide ready" => "Diapositive prête",
        "Cached · loads when nearby" => "En cache · chargée à proximité",
        "Queued for preparation" => "En attente de préparation",
        "Could not load slide" => "Échec du chargement",
        "Loading into memory…" => "Chargement en mémoire…",
        "On disk · loads nearby" => "Sur disque · chargée à proximité",
        "Not loaded · waiting" => "Non chargée · en attente",
        "No slide" => "Aucune diapositive",
        "End of slides" => "Fin des diapositives",
        "Size" => "Taille",
        "Tools" => "Outils",
        "Off" => "Désactivé",
        "Pointer" => "Pointeur",
        "Arrow" => "Flèche",
        "Highlighter" => "Surligneur",
        "Color" => "Couleur",
        "Undo" => "Annuler",
        "Clear" => "Effacer",
        "Pointer follows the current slide. Drag to place arrows or highlights. Up to 64 vector marks; slide images are never rerendered." => {
            "Le pointeur suit la diapositive actuelle. Glissez pour placer des flèches ou des surlignages. Jusqu’à 64 tracés vectoriels ; les images ne sont jamais recalculées."
        }
        "Keep header visible" => "Garder l’en-tête visible",
        "Keep footer visible" => "Garder le pied de page visible",
        "Show both" => "Afficher les deux",
        "Layout…" => "Disposition…",
        "PREVIOUS" => "PRÉCÉDENTE",
        "CURRENT" => "ACTUELLE",
        "NEXT" => "SUIVANTE",
        "PRESENTER NOTES · AUTO-FIT" => "NOTES · TAILLE AUTOMATIQUE",
        "No notes for this slide" => "Aucune note pour cette diapositive",
        "Header" => "En-tête",
        "Above slides" => "Au-dessus des diapositives",
        "Below notes" => "Sous les notes",
        "Left of notes" => "À gauche des notes",
        "Right of notes" => "À droite des notes",
        "Left of slides" => "À gauche des diapositives",
        "Right of slides" => "À droite des diapositives",
        "Elapsed time position" => "Position du temps écoulé",
        "Clock position" => "Position de l’horloge",
        "Slide progress bar" => "Barre de progression des diapositives",
        "Slide" => "Diapositive",
        "Auto-hide header" => "Masquer automatiquement l’en-tête",
        "Auto-hide footer" => "Masquer automatiquement le pied de page",
        "Menu" => "Menu",
        "Displays" => "Écrans",
        "All slides" => "Toutes les diapositives",
        "Autoplay" => "Lecture automatique",
        "Play automatically" => "Lire automatiquement",
        "Timing" => "Durée",
        "Loop" => "Boucler",
        "Timing starts when the slide is visible." => {
            "Le décompte commence lorsque la diapositive est visible."
        }
        "Layout" => "Disposition",
        "Show slides" => "Afficher les diapositives",
        "Blank slides" => "Masquer les diapositives",
        "Previous" => "Précédente",
        "Next" => "Suivante",
        "NO OUTPUT DISPLAYED" => "AUCUNE SORTIE AFFICHÉE",
        "PRESENTING" => "EN DIFFUSION",
        "PREPARING OUTPUT" => "PRÉPARATION DE LA SORTIE",
        "Presenter layout" => "Disposition du présentateur",
        "Balanced" => "Équilibrée",
        "Slide focus" => "Diapositive seule",
        "Notes first" => "Priorité aux notes",
        "Hidden bars reveal near edges. Right-click to restore." => {
            "Approchez des bords pour voir les barres. Clic droit pour les rétablir."
        }
        "Previous slide" => "Diapositive précédente",
        "Current slide" => "Diapositive actuelle",
        "Next slide" => "Diapositive suivante",
        "Double-click to edit this slide’s notes" => {
            "Double-cliquez pour modifier les notes de cette diapositive"
        }
        "Edit notes" => "Modifier les notes",
        "Saving notes…" => "Enregistrement des notes…",
        "Loading all notes and preparing sidecar…" => {
            "Lecture des notes et préparation du fichier…"
        }
        "The file contains all slides. Save changes only this slide." => {
            "Le fichier contient toutes les diapositives. Seule celle-ci sera modifiée."
        }
        "Note exceeds 256 KiB" => "La note dépasse 256 Kio",
        "Save notes" => "Enregistrer les notes",
        "Discard changes and close" => "Abandonner les modifications et fermer",
        "Reload (discard edits)" => "Recharger (abandonner les modifications)",
        "Retry" => "Réessayer",
        "Notes" => "Notes",
        "Elapsed time" => "Temps écoulé",
        "Time · HH:MM" => "Heure · HH:MM",
        "Save layout · This project" => "Enregistrer · Ce projet",
        "Save layout · All projects" => "Enregistrer · Tous les projets",
        "Use all-projects default" => "Utiliser la disposition globale",
        "Project layouts override the default." => {
            "La disposition du projet prime sur la disposition globale."
        }
        "Each screen has one role; repeat a role to mirror it." => {
            "Un rôle par écran. Répétez un rôle pour le dupliquer."
        }
        "Unused" => "Inutilisé",
        "Presenter" => "Présentateur",
        "Slides" => "Diapositives",
        "Single screen" => "Écran unique",
        "Use single-screen mode?" => "Utiliser le mode écran unique ?",
        "PDF Presenter is intended for a presenter setup with separate Presenter and Slides screens." => {
            "PDF Presenter est conçu pour une présentation avec des écrans Présentateur et Diapositives séparés."
        }
        "Only one screen was connected at startup. Single-screen mode shows slides with controls revealed at the top and bottom edges." => {
            "Un seul écran était connecté au démarrage. Le mode écran unique affiche les diapositives avec des commandes apparaissant aux bords supérieur et inférieur."
        }
        "Keep Presenter view" => "Garder la vue Présentateur",
        "Use single-screen mode" => "Utiliser le mode écran unique",
        "Window mode" => "Mode de fenêtre",
        "Follow global" => "Suivre le réglage global",
        "Always fullscreen" => "Toujours en plein écran",
        "Always windowed" => "Toujours en fenêtre",
        "Always fullscreen/windowed overrides global fullscreen commands for this screen." => {
            "Le mode toujours plein écran/fenêtre ignore les commandes globales pour cet écran."
        }
        "Swap groups" => "Inverser les groupes",
        "Window positioning can be unreliable in windowed mode, especially on Linux. Switch back to fullscreen if you experience positioning issues." => {
            "Le positionnement des fenêtres peut être peu fiable en mode fenêtré, surtout sous Linux. Revenez au plein écran si vous rencontrez des problèmes de positionnement."
        }
        "Presenter fullscreen" => "Présentateur en plein écran",
        "Slides fullscreen" => "Diapositives en plein écran",
        "Apply and remember" => "Appliquer et mémoriser",
        "Output visibility is preserved while previewing display changes." => {
            "La visibilité de la sortie est conservée pendant l’aperçu des changements."
        }
        "Open a PDF or picture folder to browse slides." => {
            "Ouvrez un PDF ou un dossier d’images pour parcourir les diapositives."
        }
        "Close" => "Fermer",
        "Slide preview" => "Aperçu de la diapositive",
        "Preview only — presentation unchanged" => "Aperçu uniquement — présentation inchangée",
        "Show on screen" => "Afficher sur l’écran",
        "Fit more slides or inspect larger previews." => {
            "Afficher plus de diapositives ou agrandir les aperçus."
        }
        "Choose where this slide browser opens." => "Choisir l’écran où s’ouvre ce navigateur.",
        "Compact" => "Compact",
        "Large" => "Grand",
        "Safe browsing" => "Navigation sans diffusion",
        "Live browsing" => "Navigation en direct",
        "Click to select. Double-click to preview. Show selected to present." => {
            "Cliquer pour sélectionner. Double-cliquer pour prévisualiser. Diffuser la sélection pour présenter."
        }
        "Changes appear immediately. Save to reuse this layout." => {
            "Les modifications sont immédiates. Enregistrez pour réutiliser cette disposition."
        }
        "Start with a preset" => "Choisir un préréglage",
        "Slides and notes" => "Diapositives et notes",
        "Hide anything you do not need to give the rest more room." => {
            "Masquez les éléments inutiles pour libérer de l’espace."
        }
        "Relative size" => "Taille relative",
        "Space (%)" => "Espace (%)",
        "Text size" => "Taille du texte",
        "Timers and progress" => "Horloges et progression",
        "Header and footer" => "En-tête et pied de page",
        "Remember this layout" => "Enregistrer cette disposition",
        "Preview size" => "Taille des aperçus",
        "Change on click" => "Diffuser au clic",
        "Presenter screen" => "Écran du présentateur",
        "Show selected" => "Diffuser la sélection",
        "Click changes presentation" => "Le clic change la présentation",
        "Click only selects — browse safely" => "Le clic sélectionne sans diffuser",
        " · CURRENT" => " · ACTUELLE",
        "Open PDF…" => "Ouvrir un PDF…",
        "Open picture folder…" => "Ouvrir un dossier d’images…",
        "Reset timer" => "Réinitialiser le chronomètre",
        "Start timer" => "Démarrer le chronomètre",
        "Pause timer" => "Mettre en pause",
        "Countdown" => "Compte à rebours",
        "Duration" => "Durée",
        "Start on first action" => "Démarrer à la première action",
        "Start begins immediately. Reset waits for the next presentation action." => {
            "Démarrer lance immédiatement. Réinitialiser attend la prochaine action de présentation."
        }
        "Running" => "En cours",
        "Stopped" => "Arrêté",
        "Waiting for first action" => "En attente de la première action",
        "Time is up" => "Temps écoulé",
        "Bar height" => "Hauteur de la barre",
        "Exit fullscreen" => "Quitter le plein écran",
        "Enter fullscreen" => "Passer en plein écran",
        "RAM budget per process (presenter + output each use this allowance):" => {
            "Budget RAM par processus (présentateur et sortie disposent chacun de ce budget) :"
        }
        "Raise only within available RAM; OS paging can cause delays. Screen rendering is capped at 4096 px." => {
            "Restez dans la RAM disponible : la mémoire virtuelle peut ralentir l’affichage. Rendu limité à 4096 px."
        }
        "Show slides automatically when opening a document" => {
            "Afficher automatiquement les diapositives à l’ouverture d’un document"
        }
        "Quit" => "Quitter",
        "Quit presentation?" => "Quitter la présentation ?",
        "Quit presentation" => "Quitter la présentation",
        "This closes Presenter and all Slides windows." => {
            "Ferme le présentateur et toutes les fenêtres de diapositives."
        }
        "Wait for notes import to finish before closing." => {
            "Attendez la fin de l’importation des notes avant de quitter."
        }
        "Save or discard notes before closing the presentation." => {
            "Enregistrez ou abandonnez les notes avant de quitter la présentation."
        }
        "Swap displays…" => "Inverser les écrans…",
        "Go to slide" => "Aller à la diapositive",
        "Enter to show · Backspace to clear" => "Entrée : diffuser · Retour arrière : effacer",
        "Enter to show; Backspace to clear" => "Entrée : diffuser ; Retour arrière : effacer",
        "Keep these display settings?" => "Conserver ces réglages d’écran ?",
        "Confirm once on any screen to apply to all displays." => {
            "Une seule confirmation sur un écran suffit pour appliquer à tous."
        }
        "Keep changes" => "Conserver",
        "Revert now" => "Rétablir maintenant",
        "Edit layout for" => "Modifier la disposition de",
        "Layout and saves apply to this display. Timer behavior is shared." => {
            "La disposition et les sauvegardes s’appliquent à cet écran. Le fonctionnement du minuteur reste partagé."
        }
        "Apply changes" => "Appliquer les modifications",
        "Selected" => "Sélection",
        "Current" => "Actuelle",
        "Blank Slides image" => "Image des diapositives masquées",
        "Custom picture selected" => "Image personnalisée sélectionnée",
        "Solid black" => "Écran noir",
        "Choose picture…" => "Choisir une image…",
        "Use solid black" => "Utiliser un écran noir",
        "A copy is saved and fitted to all Slides screens against a black background." => {
            "Une copie est conservée et ajustée à tous les écrans de diapositives, sur fond noir."
        }
        "Default PDF application" => "Application PDF par défaut",
        "Register PDF Presenter and its icon for PDF files in your Windows account." => {
            "Enregistre PDF Presenter et son icône pour les fichiers PDF de votre compte Windows."
        }
        "Set as default PDF app…" => "Définir comme application PDF par défaut…",
        "Windows will open Default Apps; select PDF Presenter for .pdf to confirm." => {
            "Windows ouvrira Applications par défaut : sélectionnez PDF Presenter pour .pdf afin de confirmer."
        }
        "Processed slides" => "Diapositives préparées",
        "Theme" => "Thème",
        "Language" => "Langue",
        _ => text,
    }
}

macro_rules! catalog {
    ($text:expr; $($source:literal => $translated:literal),* $(,)?) => {
        match $text {
            $($source => $translated,)*
            _ => $text,
        }
    };
}

fn es(text: &str) -> &str {
    catalog!(text;
        "Cancel" => "Cancelar",
        "Close" => "Cerrar",
        "Retry" => "Reintentar",
        "Quit" => "Salir",
        "Theme" => "Tema",
        "Language" => "Idioma",
        "Appearance" => "Apariencia",
        "Preferences" => "Preferencias",
        "Preferences…" => "Preferencias…",
        "Quick settings" => "Ajustes rápidos",
        "Menu" => "Menú",
        "Displays" => "Pantallas",
        "All slides" => "Todas las diapositivas",
        "Autoplay" => "Reproducción automática",
        "Play automatically" => "Reproducir automáticamente",
        "Timing" => "Duración",
        "Loop" => "Repetir",
        "Timing starts when the slide is visible." => "El tiempo comienza cuando la diapositiva está visible.",
        "Layout" => "Diseño",
        "Layout…" => "Diseño…",
        "Presenter layout" => "Diseño del presentador",
        "Presenter" => "Presentador",
        "Slides" => "Diapositivas",
        "Single screen" => "Una sola pantalla",
        "Unused" => "Sin usar",
        "Previous" => "Anterior",
        "Current" => "Actual",
        "Next" => "Siguiente",
        "PREVIOUS" => "ANTERIOR",
        "CURRENT" => "ACTUAL",
        "NEXT" => "SIGUIENTE",
        "Previous slide" => "Diapositiva anterior",
        "Current slide" => "Diapositiva actual",
        "Next slide" => "Diapositiva siguiente",
        "No slide" => "Sin diapositiva",
        "End of slides" => "Fin de las diapositivas",
        "Notes" => "Notas",
        "No notes for this slide" => "No hay notas para esta diapositiva",
        "PRESENTER NOTES · AUTO-FIT" => "NOTAS DEL PRESENTADOR · AJUSTE AUTOMÁTICO",
        "Show slides" => "Mostrar diapositivas",
        "Blank slides" => "Ocultar diapositivas",
        "NO OUTPUT DISPLAYED" => "NO SE MUESTRA NINGUNA SALIDA",
        "PRESENTING" => "PRESENTANDO",
        "PREPARING OUTPUT" => "PREPARANDO LA SALIDA",
        "Open PDF…" => "Abrir PDF…",
        "Open picture folder…" => "Abrir carpeta de imágenes…",
        "Open a PDF or picture folder to browse slides." => "Abra un PDF o una carpeta de imágenes para explorar las diapositivas.",
        "File chooser already open" => "El selector de archivos ya está abierto",
        "Import notes…" => "Importar notas…",
        "Import notes" => "Importar notas",
        "Processing notes…" => "Procesando notas…",
        "Save notes" => "Guardar notas",
        "Edit notes" => "Editar notas",
        "Saving notes…" => "Guardando notas…",
        "Empty note" => "Nota vacía",
        "Import and save notes" => "Importar y guardar notas",
        "Discard changes and close" => "Descartar cambios y cerrar",
        "Reload (discard edits)" => "Recargar (descartar cambios)",
        "Tools" => "Herramientas",
        "Off" => "Desactivado",
        "Pointer" => "Puntero",
        "Arrow" => "Flecha",
        "Highlighter" => "Resaltador",
        "Color" => "Color",
        "Size" => "Tamaño",
        "Undo" => "Deshacer",
        "Clear" => "Borrar",
        "Balanced" => "Equilibrado",
        "Slide focus" => "Prioridad a la diapositiva",
        "Notes first" => "Prioridad a las notas",
        "Start with a preset" => "Comenzar con un diseño",
        "Slides and notes" => "Diapositivas y notas",
        "Relative size" => "Tamaño relativo",
        "Space (%)" => "Espacio (%)",
        "Text size" => "Tamaño del texto",
        "Header" => "Encabezado",
        "Header and footer" => "Encabezado y pie",
        "Keep header visible" => "Mantener visible el encabezado",
        "Keep footer visible" => "Mantener visible el pie",
        "Show both" => "Mostrar ambos",
        "Auto-hide header" => "Ocultar automáticamente el encabezado",
        "Auto-hide footer" => "Ocultar automáticamente el pie",
        "Elapsed time" => "Tiempo transcurrido",
        "Time · HH:MM" => "Hora · HH:MM",
        "Timers and progress" => "Temporizadores y progreso",
        "Slide progress bar" => "Barra de progreso",
        "Reset timer" => "Reiniciar temporizador",
        "Start timer" => "Iniciar temporizador",
        "Pause timer" => "Pausar temporizador",
        "Countdown" => "Cuenta atrás",
        "Duration" => "Duración",
        "Running" => "En marcha",
        "Stopped" => "Detenido",
        "Waiting for first action" => "Esperando la primera acción",
        "Time is up" => "Se acabó el tiempo",
        "Bar height" => "Altura de la barra",
        "Each screen has one role; repeat a role to mirror it." => "Cada pantalla tiene una función; repítala para duplicarla.",
        "Window mode" => "Modo de ventana",
        "Follow global" => "Seguir ajuste global",
        "Always fullscreen" => "Siempre a pantalla completa",
        "Always windowed" => "Siempre en ventana",
        "Swap groups" => "Intercambiar grupos",
        "Presenter fullscreen" => "Presentador a pantalla completa",
        "Slides fullscreen" => "Diapositivas a pantalla completa",
        "Apply and remember" => "Aplicar y recordar",
        "Use single-screen mode?" => "¿Usar el modo de una sola pantalla?",
        "Keep Presenter view" => "Mantener la vista Presentador",
        "Use single-screen mode" => "Usar el modo de una sola pantalla",
        "Exit fullscreen" => "Salir de pantalla completa",
        "Enter fullscreen" => "Entrar en pantalla completa",
        "Slide preview" => "Vista previa de la diapositiva",
        "Preview only — presentation unchanged" => "Solo vista previa — la presentación no cambia",
        "Show on screen" => "Mostrar en la pantalla",
        "Preview size" => "Tamaño de la vista previa",
        "Compact" => "Compacto",
        "Large" => "Grande",
        "Safe browsing" => "Exploración segura",
        "Live browsing" => "Exploración en directo",
        "Change on click" => "Cambiar al hacer clic",
        "Show selected" => "Mostrar selección",
        "Page background" => "Fondo de página",
        "Fit padding color" => "Color del margen",
        "Enable page zoom" => "Activar zoom de página",
        "Page zoom" => "Zoom de página",
        "Reset zoom" => "Restablecer zoom",
        "Disable zoom" => "Desactivar zoom",
        "Keep zoom enabled" => "Mantener zoom activado",
        "Memory and cache" => "Memoria y caché",
        "Disk cache" => "Caché de disco",
        "Storage and preparation" => "Almacenamiento y preparación",
        "Application executable" => "Ejecutable de la aplicación",
        "This is the running portable executable. Use its containing folder to find or remove the application." => "Este es el ejecutable portátil en uso. Utilice la carpeta que lo contiene para encontrar o eliminar la aplicación.",
        "Storage locations" => "Ubicaciones de almacenamiento",
        "Preparing slide…" => "Preparando diapositiva…",
        "Slide ready" => "Diapositiva lista",
        "Loading into memory…" => "Cargando en memoria…",
        "Could not load slide" => "No se pudo cargar la diapositiva",
        "Quit presentation?" => "¿Salir de la presentación?",
        "Quit presentation" => "Salir de la presentación",
        "This closes Presenter and all Slides windows." => "Esto cierra el Presentador y todas las ventanas de diapositivas.",
        "Swap displays…" => "Intercambiar pantallas…", "Go to slide" => "Ir a la diapositiva",
        "Enter to show · Backspace to clear" => "Intro: mostrar · Retroceso: borrar",
        "Enter to show; Backspace to clear" => "Intro: mostrar; Retroceso: borrar",
        "Keep these display settings?" => "¿Conservar estos ajustes de pantalla?",
        "Confirm once on any screen to apply to all displays." => "Confirme una vez en cualquier pantalla para aplicar a todas.",
        "Keep changes" => "Conservar cambios", "Revert now" => "Revertir ahora",
        "Edit layout for" => "Editar diseño de", "Apply changes" => "Aplicar cambios",
        "Selected" => "Seleccionada", "Blank Slides image" => "Imagen de diapositivas ocultas",
        "Custom picture selected" => "Imagen personalizada seleccionada", "Solid black" => "Negro sólido",
        "Choose picture…" => "Elegir imagen…", "Use solid black" => "Usar negro sólido",
        "Processed slides" => "Diapositivas procesadas"
    )
}

fn ja(text: &str) -> &str {
    catalog!(text;
        "Cancel" => "キャンセル",
        "Close" => "閉じる",
        "Retry" => "再試行",
        "Quit" => "終了",
        "Theme" => "テーマ",
        "Language" => "言語",
        "Appearance" => "外観",
        "Preferences" => "設定",
        "Preferences…" => "設定…",
        "Quick settings" => "クイック設定",
        "Menu" => "メニュー",
        "Displays" => "ディスプレイ",
        "All slides" => "すべてのスライド",
        "Autoplay" => "自動再生",
        "Play automatically" => "自動的に再生",
        "Timing" => "表示時間",
        "Loop" => "ループ",
        "Timing starts when the slide is visible." => "スライドが表示されてから時間の計測を開始します。",
        "Layout" => "レイアウト",
        "Layout…" => "レイアウト…",
        "Presenter layout" => "発表者レイアウト",
        "Presenter" => "発表者",
        "Slides" => "スライド",
        "Single screen" => "シングルスクリーン",
        "Unused" => "未使用",
        "Previous" => "前へ",
        "Current" => "現在",
        "Next" => "次へ",
        "PREVIOUS" => "前のスライド",
        "CURRENT" => "現在のスライド",
        "NEXT" => "次のスライド",
        "Previous slide" => "前のスライド",
        "Current slide" => "現在のスライド",
        "Next slide" => "次のスライド",
        "No slide" => "スライドなし",
        "End of slides" => "スライドの終わり",
        "Notes" => "ノート",
        "No notes for this slide" => "このスライドにはノートがありません",
        "PRESENTER NOTES · AUTO-FIT" => "発表者ノート · 自動調整",
        "Show slides" => "スライドを表示",
        "Blank slides" => "スライドを非表示",
        "NO OUTPUT DISPLAYED" => "出力は表示されていません",
        "PRESENTING" => "プレゼンテーション中",
        "PREPARING OUTPUT" => "出力を準備中",
        "Open PDF…" => "PDFを開く…",
        "Open picture folder…" => "画像フォルダーを開く…",
        "Open a PDF or picture folder to browse slides." => "PDFまたは画像フォルダーを開いてスライドを参照します。",
        "File chooser already open" => "ファイル選択画面はすでに開いています",
        "Import notes…" => "ノートをインポート…",
        "Import notes" => "ノートをインポート",
        "Processing notes…" => "ノートを処理中…",
        "Save notes" => "ノートを保存",
        "Edit notes" => "ノートを編集",
        "Saving notes…" => "ノートを保存中…",
        "Empty note" => "空のノート",
        "Import and save notes" => "ノートをインポートして保存",
        "Discard changes and close" => "変更を破棄して閉じる",
        "Reload (discard edits)" => "再読み込み（編集を破棄）",
        "Tools" => "ツール",
        "Off" => "オフ",
        "Pointer" => "ポインター",
        "Arrow" => "矢印",
        "Highlighter" => "蛍光ペン",
        "Color" => "色",
        "Size" => "サイズ",
        "Undo" => "元に戻す",
        "Clear" => "消去",
        "Balanced" => "バランス",
        "Slide focus" => "スライド優先",
        "Notes first" => "ノート優先",
        "Start with a preset" => "プリセットから開始",
        "Slides and notes" => "スライドとノート",
        "Relative size" => "相対サイズ",
        "Space (%)" => "領域（%）",
        "Text size" => "文字サイズ",
        "Header" => "ヘッダー",
        "Header and footer" => "ヘッダーとフッター",
        "Keep header visible" => "ヘッダーを常に表示",
        "Keep footer visible" => "フッターを常に表示",
        "Show both" => "両方を表示",
        "Auto-hide header" => "ヘッダーを自動的に隠す",
        "Auto-hide footer" => "フッターを自動的に隠す",
        "Elapsed time" => "経過時間",
        "Time · HH:MM" => "時刻 · HH:MM",
        "Timers and progress" => "タイマーと進行状況",
        "Slide progress bar" => "スライド進行バー",
        "Reset timer" => "タイマーをリセット",
        "Start timer" => "タイマーを開始",
        "Pause timer" => "タイマーを一時停止",
        "Countdown" => "カウントダウン",
        "Duration" => "時間",
        "Running" => "実行中",
        "Stopped" => "停止中",
        "Waiting for first action" => "最初の操作を待機中",
        "Time is up" => "時間です",
        "Bar height" => "バーの高さ",
        "Each screen has one role; repeat a role to mirror it." => "各画面に役割を割り当てます。同じ役割を選ぶとミラー表示します。",
        "Window mode" => "ウィンドウモード",
        "Follow global" => "全体設定に従う",
        "Always fullscreen" => "常に全画面",
        "Always windowed" => "常にウィンドウ表示",
        "Swap groups" => "グループを入れ替え",
        "Presenter fullscreen" => "発表者を全画面表示",
        "Slides fullscreen" => "スライドを全画面表示",
        "Apply and remember" => "適用して保存",
        "Use single-screen mode?" => "シングルスクリーンモードを使用しますか？",
        "Keep Presenter view" => "発表者ビューを維持",
        "Use single-screen mode" => "シングルスクリーンモードを使用",
        "Exit fullscreen" => "全画面表示を終了",
        "Enter fullscreen" => "全画面表示にする",
        "Slide preview" => "スライドプレビュー",
        "Preview only — presentation unchanged" => "プレビューのみ — プレゼンテーションは変更されません",
        "Show on screen" => "表示する画面",
        "Preview size" => "プレビューサイズ",
        "Compact" => "コンパクト",
        "Large" => "大きい",
        "Safe browsing" => "安全な閲覧",
        "Live browsing" => "ライブ閲覧",
        "Change on click" => "クリックで切り替え",
        "Show selected" => "選択したものを表示",
        "Page background" => "ページ背景",
        "Fit padding color" => "余白の色",
        "Enable page zoom" => "ページズームを有効化",
        "Page zoom" => "ページズーム",
        "Reset zoom" => "ズームをリセット",
        "Disable zoom" => "ズームを無効化",
        "Keep zoom enabled" => "ズームを有効のままにする",
        "Memory and cache" => "メモリとキャッシュ",
        "Disk cache" => "ディスクキャッシュ",
        "Storage and preparation" => "保存場所と準備",
        "Application executable" => "アプリケーションの実行ファイル",
        "This is the running portable executable. Use its containing folder to find or remove the application." => "現在実行中のポータブル実行ファイルです。アプリを見つけたり削除したりするには、このファイルがあるフォルダーを使用してください。",
        "Storage locations" => "保存場所",
        "Preparing slide…" => "スライドを準備中…",
        "Slide ready" => "スライドの準備完了",
        "Loading into memory…" => "メモリに読み込み中…",
        "Could not load slide" => "スライドを読み込めませんでした",
        "Quit presentation?" => "プレゼンテーションを終了しますか？",
        "Quit presentation" => "プレゼンテーションを終了",
        "This closes Presenter and all Slides windows." => "発表者画面とすべてのスライド画面を閉じます。",
        "Swap displays…" => "画面を入れ替え…", "Go to slide" => "スライドへ移動",
        "Enter to show · Backspace to clear" => "Enterで表示 · Backspaceで消去",
        "Enter to show; Backspace to clear" => "Enterで表示、Backspaceで消去",
        "Keep these display settings?" => "このディスプレイ設定を維持しますか？",
        "Confirm once on any screen to apply to all displays." => "いずれかの画面で一度確認すると、すべてに適用されます。",
        "Keep changes" => "変更を維持", "Revert now" => "今すぐ元に戻す",
        "Edit layout for" => "レイアウトの編集対象", "Apply changes" => "変更を適用",
        "Selected" => "選択", "Blank Slides image" => "スライド非表示時の画像",
        "Custom picture selected" => "カスタム画像を選択済み", "Solid black" => "黒一色",
        "Choose picture…" => "画像を選択…", "Use solid black" => "黒一色を使用",
        "Processed slides" => "処理済みスライド"
    )
}

fn zh_cn(text: &str) -> &str {
    catalog!(text;
        "Cancel" => "取消", "Close" => "关闭", "Retry" => "重试", "Quit" => "退出",
        "Theme" => "主题", "Language" => "语言", "Appearance" => "外观",
        "Preferences" => "首选项", "Preferences…" => "首选项…", "Quick settings" => "快速设置",
        "Menu" => "菜单", "Displays" => "显示器", "All slides" => "所有幻灯片",
        "Autoplay" => "自动播放", "Play automatically" => "自动播放幻灯片", "Timing" => "播放时长", "Loop" => "循环",
        "Timing starts when the slide is visible." => "幻灯片显示后才开始计时。",
        "Layout" => "布局", "Layout…" => "布局…", "Presenter layout" => "演讲者布局",
        "Presenter" => "演讲者", "Slides" => "幻灯片", "Single screen" => "单屏模式", "Unused" => "未使用",
        "Previous" => "上一张", "Current" => "当前", "Next" => "下一张",
        "PREVIOUS" => "上一张", "CURRENT" => "当前", "NEXT" => "下一张",
        "Previous slide" => "上一张幻灯片", "Current slide" => "当前幻灯片", "Next slide" => "下一张幻灯片",
        "No slide" => "无幻灯片", "End of slides" => "幻灯片结束", "Notes" => "备注",
        "No notes for this slide" => "此幻灯片没有备注", "PRESENTER NOTES · AUTO-FIT" => "演讲者备注 · 自动适应",
        "Show slides" => "显示幻灯片", "Blank slides" => "隐藏幻灯片",
        "NO OUTPUT DISPLAYED" => "未显示输出", "PRESENTING" => "正在演示", "PREPARING OUTPUT" => "正在准备输出",
        "Open PDF…" => "打开 PDF…", "Open picture folder…" => "打开图片文件夹…",
        "Open a PDF or picture folder to browse slides." => "打开 PDF 或图片文件夹以浏览幻灯片。",
        "File chooser already open" => "文件选择器已打开", "Import notes…" => "导入备注…",
        "Import notes" => "导入备注", "Processing notes…" => "正在处理备注…", "Save notes" => "保存备注",
        "Edit notes" => "编辑备注", "Saving notes…" => "正在保存备注…", "Empty note" => "空备注",
        "Import and save notes" => "导入并保存备注", "Discard changes and close" => "放弃更改并关闭",
        "Reload (discard edits)" => "重新加载（放弃编辑）", "Tools" => "工具", "Off" => "关闭",
        "Pointer" => "指针", "Arrow" => "箭头", "Highlighter" => "荧光笔", "Color" => "颜色",
        "Size" => "大小", "Undo" => "撤销", "Clear" => "清除", "Balanced" => "均衡",
        "Slide focus" => "幻灯片优先", "Notes first" => "备注优先", "Start with a preset" => "从预设开始",
        "Slides and notes" => "幻灯片和备注", "Relative size" => "相对大小", "Space (%)" => "空间 (%)",
        "Text size" => "文字大小", "Header" => "页眉", "Header and footer" => "页眉和页脚",
        "Keep header visible" => "保持页眉可见", "Keep footer visible" => "保持页脚可见", "Show both" => "两者都显示",
        "Auto-hide header" => "自动隐藏页眉", "Auto-hide footer" => "自动隐藏页脚",
        "Elapsed time" => "已用时间", "Time · HH:MM" => "时间 · HH:MM", "Timers and progress" => "计时器和进度",
        "Slide progress bar" => "幻灯片进度条", "Reset timer" => "重置计时器", "Start timer" => "启动计时器",
        "Pause timer" => "暂停计时器", "Countdown" => "倒计时", "Duration" => "时长", "Running" => "运行中",
        "Stopped" => "已停止", "Waiting for first action" => "等待首次操作", "Time is up" => "时间到",
        "Bar height" => "进度条高度", "Each screen has one role; repeat a role to mirror it." => "每个屏幕分配一个角色；重复角色可进行镜像显示。",
        "Window mode" => "窗口模式", "Follow global" => "跟随全局设置", "Always fullscreen" => "始终全屏",
        "Always windowed" => "始终窗口化", "Swap groups" => "交换分组", "Presenter fullscreen" => "演讲者全屏",
        "Slides fullscreen" => "幻灯片全屏", "Apply and remember" => "应用并记住",
        "Use single-screen mode?" => "使用单屏模式？", "Keep Presenter view" => "保留演讲者视图",
        "Use single-screen mode" => "使用单屏模式", "Exit fullscreen" => "退出全屏", "Enter fullscreen" => "进入全屏",
        "Slide preview" => "幻灯片预览", "Preview only — presentation unchanged" => "仅预览 — 演示内容不变",
        "Show on screen" => "显示在屏幕", "Preview size" => "预览大小", "Compact" => "紧凑", "Large" => "大",
        "Safe browsing" => "安全浏览", "Live browsing" => "实时浏览", "Change on click" => "点击时切换",
        "Show selected" => "显示所选项", "Page background" => "页面背景", "Fit padding color" => "边距颜色",
        "Enable page zoom" => "启用页面缩放", "Page zoom" => "页面缩放", "Reset zoom" => "重置缩放",
        "Disable zoom" => "禁用缩放", "Keep zoom enabled" => "保持缩放启用", "Memory and cache" => "内存和缓存",
        "Disk cache" => "磁盘缓存", "Storage and preparation" => "存储和准备", "Storage locations" => "存储位置",
        "Application executable" => "应用程序可执行文件",
        "This is the running portable executable. Use its containing folder to find or remove the application." => "这是当前运行的便携版可执行文件。可通过其所在文件夹查找或删除本应用程序。",
        "Preparing slide…" => "正在准备幻灯片…", "Slide ready" => "幻灯片已就绪", "Loading into memory…" => "正在加载到内存…",
        "Could not load slide" => "无法加载幻灯片", "Quit presentation?" => "退出演示？",
        "Quit presentation" => "退出演示", "This closes Presenter and all Slides windows." => "这将关闭演讲者窗口和所有幻灯片窗口。",
        "Swap displays…" => "交换显示器…", "Go to slide" => "转到幻灯片",
        "Enter to show · Backspace to clear" => "Enter 显示 · Backspace 清除",
        "Enter to show; Backspace to clear" => "Enter 显示；Backspace 清除",
        "Keep these display settings?" => "保留这些显示设置？",
        "Confirm once on any screen to apply to all displays." => "在任意屏幕确认一次即可应用到所有显示器。",
        "Keep changes" => "保留更改", "Revert now" => "立即还原", "Edit layout for" => "编辑布局：",
        "Apply changes" => "应用更改", "Selected" => "已选择", "Blank Slides image" => "隐藏幻灯片时的图片",
        "Custom picture selected" => "已选择自定义图片", "Solid black" => "纯黑",
        "Choose picture…" => "选择图片…", "Use solid black" => "使用纯黑", "Processed slides" => "已处理的幻灯片"
    )
}

fn de(text: &str) -> &str {
    catalog!(text;
        "Cancel" => "Abbrechen", "Close" => "Schließen", "Retry" => "Erneut versuchen", "Quit" => "Beenden",
        "Theme" => "Design", "Language" => "Sprache", "Appearance" => "Darstellung",
        "Preferences" => "Einstellungen", "Preferences…" => "Einstellungen…", "Quick settings" => "Schnelleinstellungen",
        "Menu" => "Menü", "Displays" => "Bildschirme", "All slides" => "Alle Folien", "Layout" => "Layout",
        "Autoplay" => "Automatische Wiedergabe", "Play automatically" => "Automatisch abspielen", "Timing" => "Anzeigedauer", "Loop" => "Wiederholen",
        "Timing starts when the slide is visible." => "Die Zeitmessung beginnt, sobald die Folie sichtbar ist.",
        "Layout…" => "Layout…", "Presenter layout" => "Präsentatorlayout", "Presenter" => "Präsentator",
        "Slides" => "Folien", "Single screen" => "Einzelbildschirm", "Unused" => "Unbenutzt",
        "Previous" => "Zurück", "Current" => "Aktuell", "Next" => "Weiter", "PREVIOUS" => "VORHERIGE",
        "CURRENT" => "AKTUELL", "NEXT" => "NÄCHSTE", "Previous slide" => "Vorherige Folie",
        "Current slide" => "Aktuelle Folie", "Next slide" => "Nächste Folie", "No slide" => "Keine Folie",
        "End of slides" => "Ende der Folien", "Notes" => "Notizen", "No notes for this slide" => "Keine Notizen für diese Folie",
        "Show slides" => "Folien anzeigen", "Blank slides" => "Folien ausblenden", "NO OUTPUT DISPLAYED" => "KEINE AUSGABE ANGEZEIGT",
        "PRESENTING" => "PRÄSENTATION LÄUFT", "PREPARING OUTPUT" => "AUSGABE WIRD VORBEREITET",
        "Open PDF…" => "PDF öffnen…", "Open picture folder…" => "Bilderordner öffnen…", "Import notes…" => "Notizen importieren…",
        "Save notes" => "Notizen speichern", "Edit notes" => "Notizen bearbeiten", "Tools" => "Werkzeuge", "Off" => "Aus",
        "Pointer" => "Zeiger", "Arrow" => "Pfeil", "Highlighter" => "Textmarker", "Color" => "Farbe", "Size" => "Größe",
        "Undo" => "Rückgängig", "Clear" => "Löschen", "Balanced" => "Ausgewogen", "Slide focus" => "Folienfokus",
        "Notes first" => "Notizen zuerst", "Slides and notes" => "Folien und Notizen", "Header" => "Kopfzeile",
        "Header and footer" => "Kopf- und Fußzeile", "Keep header visible" => "Kopfzeile sichtbar lassen",
        "Keep footer visible" => "Fußzeile sichtbar lassen", "Elapsed time" => "Verstrichene Zeit", "Countdown" => "Countdown",
        "Duration" => "Dauer", "Reset timer" => "Timer zurücksetzen", "Start timer" => "Timer starten",
        "Pause timer" => "Timer pausieren", "Window mode" => "Fenstermodus", "Follow global" => "Globaler Einstellung folgen",
        "Always fullscreen" => "Immer Vollbild", "Always windowed" => "Immer im Fenster", "Swap groups" => "Gruppen tauschen",
        "Presenter fullscreen" => "Präsentator im Vollbild", "Slides fullscreen" => "Folien im Vollbild",
        "Apply and remember" => "Anwenden und merken", "Exit fullscreen" => "Vollbild verlassen",
        "Enter fullscreen" => "Vollbild öffnen", "Slide preview" => "Folienvorschau", "Preview size" => "Vorschaugröße",
        "Compact" => "Kompakt", "Large" => "Groß", "Safe browsing" => "Sicheres Durchsuchen",
        "Live browsing" => "Live-Durchsuchen", "Change on click" => "Bei Klick wechseln", "Show selected" => "Auswahl zeigen",
        "Enable page zoom" => "Seitenzoom aktivieren", "Page zoom" => "Seitenzoom", "Reset zoom" => "Zoom zurücksetzen",
        "Memory and cache" => "Speicher und Cache", "Disk cache" => "Datenträgercache",
        "Storage and preparation" => "Speicher und Vorbereitung", "Application executable" => "Anwendungsdatei",
        "This is the running portable executable. Use its containing folder to find or remove the application." => "Dies ist die aktuell ausgeführte portable Anwendungsdatei. Über ihren Ordner können Sie die Anwendung finden oder entfernen.",
        "Quit presentation?" => "Präsentation beenden?",
        "Quit presentation" => "Präsentation beenden", "This closes Presenter and all Slides windows." => "Dadurch werden der Präsentator und alle Folienfenster geschlossen.",
        "Swap displays…" => "Bildschirme tauschen…", "Go to slide" => "Zu Folie springen",
        "Keep these display settings?" => "Diese Bildschirmeinstellungen behalten?", "Keep changes" => "Änderungen behalten",
        "Revert now" => "Jetzt zurücksetzen", "Apply changes" => "Änderungen anwenden", "Selected" => "Ausgewählt",
        "Processed slides" => "Verarbeitete Folien"
    )
}

fn pt_br(text: &str) -> &str {
    catalog!(text;
        "Cancel" => "Cancelar", "Close" => "Fechar", "Retry" => "Tentar novamente", "Quit" => "Sair",
        "Theme" => "Tema", "Language" => "Idioma", "Appearance" => "Aparência",
        "Preferences" => "Preferências", "Preferences…" => "Preferências…", "Quick settings" => "Configurações rápidas",
        "Menu" => "Menu", "Displays" => "Telas", "All slides" => "Todos os slides", "Layout" => "Layout",
        "Autoplay" => "Reprodução automática", "Play automatically" => "Reproduzir automaticamente", "Timing" => "Duração", "Loop" => "Repetir",
        "Timing starts when the slide is visible." => "A contagem começa quando o slide está visível.",
        "Layout…" => "Layout…", "Presenter layout" => "Layout do apresentador", "Presenter" => "Apresentador",
        "Slides" => "Slides", "Single screen" => "Tela única", "Unused" => "Não usada", "Previous" => "Anterior",
        "Current" => "Atual", "Next" => "Próximo", "PREVIOUS" => "ANTERIOR", "CURRENT" => "ATUAL", "NEXT" => "PRÓXIMO",
        "Previous slide" => "Slide anterior", "Current slide" => "Slide atual", "Next slide" => "Próximo slide",
        "No slide" => "Sem slide", "End of slides" => "Fim dos slides", "Notes" => "Notas",
        "No notes for this slide" => "Sem notas para este slide", "Show slides" => "Mostrar slides",
        "Blank slides" => "Ocultar slides", "NO OUTPUT DISPLAYED" => "NENHUMA SAÍDA EXIBIDA",
        "PRESENTING" => "APRESENTANDO", "PREPARING OUTPUT" => "PREPARANDO A SAÍDA",
        "Open PDF…" => "Abrir PDF…", "Open picture folder…" => "Abrir pasta de imagens…",
        "Import notes…" => "Importar notas…", "Save notes" => "Salvar notas", "Edit notes" => "Editar notas",
        "Tools" => "Ferramentas", "Off" => "Desativado", "Pointer" => "Ponteiro", "Arrow" => "Seta",
        "Highlighter" => "Marca-texto", "Color" => "Cor", "Size" => "Tamanho", "Undo" => "Desfazer", "Clear" => "Limpar",
        "Balanced" => "Equilibrado", "Slide focus" => "Foco no slide", "Notes first" => "Notas primeiro",
        "Slides and notes" => "Slides e notas", "Header" => "Cabeçalho", "Header and footer" => "Cabeçalho e rodapé",
        "Keep header visible" => "Manter cabeçalho visível", "Keep footer visible" => "Manter rodapé visível",
        "Elapsed time" => "Tempo decorrido", "Countdown" => "Contagem regressiva", "Duration" => "Duração",
        "Reset timer" => "Reiniciar cronômetro", "Start timer" => "Iniciar cronômetro", "Pause timer" => "Pausar cronômetro",
        "Window mode" => "Modo de janela", "Follow global" => "Seguir configuração global",
        "Always fullscreen" => "Sempre em tela cheia", "Always windowed" => "Sempre em janela",
        "Swap groups" => "Trocar grupos", "Presenter fullscreen" => "Apresentador em tela cheia",
        "Slides fullscreen" => "Slides em tela cheia", "Apply and remember" => "Aplicar e lembrar",
        "Exit fullscreen" => "Sair da tela cheia", "Enter fullscreen" => "Entrar em tela cheia",
        "Slide preview" => "Prévia do slide", "Preview size" => "Tamanho da prévia", "Compact" => "Compacto", "Large" => "Grande",
        "Safe browsing" => "Navegação segura", "Live browsing" => "Navegação ao vivo", "Change on click" => "Alterar ao clicar",
        "Show selected" => "Mostrar selecionado", "Enable page zoom" => "Ativar zoom da página", "Page zoom" => "Zoom da página",
        "Reset zoom" => "Redefinir zoom", "Memory and cache" => "Memória e cache", "Disk cache" => "Cache em disco",
        "Storage and preparation" => "Armazenamento e preparação", "Application executable" => "Executável do aplicativo",
        "This is the running portable executable. Use its containing folder to find or remove the application." => "Este é o executável portátil em uso. Use a pasta que o contém para encontrar ou remover o aplicativo.",
        "Quit presentation?" => "Sair da apresentação?",
        "Quit presentation" => "Sair da apresentação", "This closes Presenter and all Slides windows." => "Isso fecha o Apresentador e todas as janelas de slides.",
        "Swap displays…" => "Trocar telas…", "Go to slide" => "Ir para o slide",
        "Keep these display settings?" => "Manter estas configurações de tela?", "Keep changes" => "Manter alterações",
        "Revert now" => "Reverter agora", "Apply changes" => "Aplicar alterações", "Selected" => "Selecionado",
        "Processed slides" => "Slides processados"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_locales_resolve_supported_languages_without_misclassifying_scripts() {
        for (locale, expected) in [
            ("fr_CA.UTF-8", ResolvedLanguage::Fr),
            ("es-MX", ResolvedLanguage::Es),
            ("ja_JP", ResolvedLanguage::Ja),
            ("zh_CN.UTF-8", ResolvedLanguage::ZhCn),
            ("zh-Hans-SG", ResolvedLanguage::ZhCn),
            ("de-DE", ResolvedLanguage::De),
            ("pt-BR", ResolvedLanguage::PtBr),
            ("zh-TW", ResolvedLanguage::En),
            ("zh-Hant", ResolvedLanguage::En),
            ("pt-PT", ResolvedLanguage::En),
            ("C", ResolvedLanguage::En),
        ] {
            assert_eq!(resolve_system_locale(Some(locale)), expected, "{locale}");
        }
    }

    #[test]
    fn catalogs_translate_core_presenter_controls_and_fall_back_to_english() {
        let ctx = egui::Context::default();
        for (language, expected, autoplay) in [
            (Language::Fr, "Écrans", "Lecture automatique"),
            (Language::Es, "Pantallas", "Reproducción automática"),
            (Language::Ja, "ディスプレイ", "自動再生"),
            (Language::ZhCn, "显示器", "自动播放"),
            (Language::De, "Bildschirme", "Automatische Wiedergabe"),
            (Language::PtBr, "Telas", "Reprodução automática"),
        ] {
            apply(&ctx, &language);
            assert_eq!(tr(&ctx, "Displays"), expected);
            assert_eq!(tr(&ctx, "Autoplay"), autoplay);
            assert_eq!(
                tr(&ctx, "untranslated diagnostic"),
                "untranslated diagnostic"
            );
        }
    }

    #[test]
    fn embedded_cjk_faces_render_selected_interface_scripts() {
        let ctx = egui::Context::default();
        for (language, sample) in [
            (Language::Ja, "日本語スライド"),
            (Language::ZhCn, "简体中文幻灯片"),
        ] {
            apply(&ctx, &language);
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.fonts_mut(|fonts| {
                    let galley = fonts.layout_no_wrap(
                        sample.into(),
                        egui::FontId::proportional(18.0),
                        egui::Color32::WHITE,
                    );
                    assert_eq!(galley.text(), sample);
                    assert!(
                        galley.rows[0]
                            .glyphs
                            .iter()
                            .all(|glyph| !glyph.uv_rect.is_nothing()),
                        "missing glyph in {language:?}"
                    );
                });
            });
        }
    }
}

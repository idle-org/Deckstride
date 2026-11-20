//! Interface catalog; diagnostic messages from workers remain in their original language.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    En,
    Fr,
}
impl Language {
    pub const ALL: [Self; 3] = [Self::System, Self::En, Self::Fr];
    pub fn name(self) -> &'static str {
        match self {
            Self::System => "System / Système",
            Self::En => "English",
            Self::Fr => "Français",
        }
    }
    pub fn french(self) -> bool {
        match self {
            Self::Fr => true,
            Self::En => false,
            Self::System => system_french(),
        }
    }
}
fn system_french() -> bool {
    static FRENCH: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FRENCH.get_or_init(|| {
        sys_locale::get_locale().is_some_and(|s| s.to_ascii_lowercase().starts_with("fr"))
    })
}
pub fn apply(ctx: &egui::Context, language: &Language) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("french"), language.french()));
}
pub fn tr<'a>(ctx: &egui::Context, text: &'a str) -> &'a str {
    if !ctx.data(|d| d.get_temp::<bool>(egui::Id::new("french")).unwrap_or(false)) {
        return text;
    }
    match text {
        "File chooser already open" => "Sélecteur de fichiers déjà ouvert",
        "Page background" => "Fond de page",
        "Fit padding color" => "Couleur des marges",
        "Page background fills transparency; fit padding fills space around the slide." => {
            "Le fond de page remplit les zones transparentes ; les marges remplissent l’espace autour de la diapositive."
        }
        "Appearance" => "Apparence",
        "Preferences" => "Préférences",
        "Storage locations" => "Emplacements de stockage",
        "Preparing slide…" => "Préparation de la diapositive…",
        "No slide" => "Aucune diapositive",
        "End of slides" => "Fin des diapositives",
        "Size" => "Taille",
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
        "Layout" => "Disposition",
        "Show presentation" => "Afficher la présentation",
        "Blank presentation" => "Masquer la présentation",
        "Previous" => "Précédente",
        "Next" => "Suivante",
        "NO OUTPUT DISPLAYED" => "AUCUNE SORTIE AFFICHÉE",
        "PRESENTING" => "EN DIFFUSION",
        "PREPARING OUTPUT" => "PRÉPARATION DE LA SORTIE",
        "Your presenter view" => "Votre vue présentateur",
        "Balanced" => "Équilibrée",
        "Slide focus" => "Diapositive seule",
        "Notes first" => "Priorité aux notes",
        "Hidden bars reveal near edges. Right-click to restore." => {
            "Approchez des bords pour voir les barres. Clic droit pour les rétablir."
        }
        "Previous slide" => "Diapositive précédente",
        "Current slide" => "Diapositive actuelle",
        "Next slide" => "Diapositive suivante",
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
        "Presentation" => "Présentation",
        "Swap groups" => "Inverser les groupes",
        "Window positioning can be unreliable in windowed mode, especially on Linux. Switch back to fullscreen if you experience positioning issues." => {
            "Le positionnement des fenêtres peut être peu fiable en mode fenêtré, surtout sous Linux. Revenez au plein écran si vous rencontrez des problèmes de positionnement."
        }
        "Presenter fullscreen" => "Présentateur en plein écran",
        "Presentation fullscreen" => "Présentation en plein écran",
        "Apply and remember" => "Appliquer et mémoriser",
        "Applying a setup blanks output. Show when destinations are verified." => {
            "Appliquer masque la sortie. Vérifiez les destinations avant de la réactiver."
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
        "Exit fullscreen" => "Quitter le plein écran",
        "Enter fullscreen" => "Passer en plein écran",
        "RAM budget per process (presenter + output each use this allowance):" => {
            "Budget RAM par processus (présentateur et sortie disposent chacun de ce budget) :"
        }
        "Raise only within available RAM; OS paging can cause delays. Screen rendering is capped at 4096 px." => {
            "Restez dans la RAM disponible : la mémoire virtuelle peut ralentir l’affichage. Rendu limité à 4096 px."
        }
        "Show presentation automatically when opening a document" => {
            "Afficher automatiquement la présentation à l’ouverture d’un document"
        }
        "Quit" => "Quitter",
        "Theme" => "Thème",
        "Language" => "Langue",
        _ => text,
    }
}

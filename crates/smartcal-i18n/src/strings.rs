use smartcal_core::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    AppTitle,
    Subtitle,
    QuickCalibrate,
    QuickCalibrateHint,
    DetectHardware,
    DetectHardwareDesc,
    StartCalibration,
    StartCalibrationDesc,
    TestTouch,
    TestTouchDesc,
    Profiles,
    ProfilesDesc,
    Settings,
    SettingsDesc,
    Diagnostics,
    DiagnosticsDesc,
    Home,
    PreviousCalibrationFound,
    Apply,
    Recalibrate,
    Dismiss,
    Step1Title,
    Step2Title,
    Step3Title,
    StepDisplayShort,
    StepDeviceShort,
    StepMethodShort,
    Next,
    Back,
    Cancel,
    Finish,
    DisplaysHeading,
    InputDevicesHeading,
    ProjectorBadge,
    PrimaryBadge,
    RecommendedBadge,
    ReadyBadge,
    SetupNeededBadge,
    TargetDisplayLabel,
    TargetDeviceLabel,
    SessionLabel,
    PermissionsLabel,
    ClickMonitorHint,
    LiveTouchSignalPrompt,
    LiveTouchSignalReceived,
    NoDevicesDetected,
    PatternPreviewTitle,
    ProjectorMode,
    ProjectorModeHint,
    Cal4Point,
    Cal9Point,
    Cal16Point,
    Cal25Point,
    CalAdvanced,
    DefaultBadge,
    MethodLabel,
    MethodHomography,
    MethodHomographyRansac,
    MethodAffine,
    MethodPoly2,
    MethodPoly3,
    MethodHybridTps,
    TouchTargetPrompt,
    CollectingSamples,
    PointProgress,
    TouchInaccurateError,
    CalibrationResults,
    AccuracyVectorMapTitle,
    AccuracyMapLegend,
    PointRowLabel,
    InliersLabel,
    ExpectedPos,
    ActualPos,
    PixelError,
    AverageError,
    MaximumError,
    RmsError,
    CalibrationQualityLabel,
    QualityExcellent,
    QualityGood,
    QualityAcceptable,
    QualityPoor,
    QualityInsufficient,
    ProfileNameLabel,
    ApplyCalibration,
    ApplyOnStartup,
    SaveProfile,
    ResetCalibration,
    RestorePreviousCalibration,
    TestModeHeading,
    TestModeHint,
    ClearPath,
    ExitTestMode,
    WindowedLabel,
    FullscreenLabel,
    GridDensityLabel,
    RawCoords,
    ScreenCoords,
    ErrorCoords,
    SettingsTargetSize,
    SettingsHighContrast,
    SettingsTargetDwell,
    SettingsSampleCount,
    SettingsMinDuration,
    SettingsMaxDeviation,
    SettingsDiscardInitial,
    SettingsTouchSensitivity,
    SettingsLanguage,
    SettingsTheme,
    LightMode,
    DarkMode,
    ExportDiagnostics,
    OpenLogs,
    InstallUdevRule,
    UdevSectionTitle,
    UdevInstallSuccess,
    UdevInstallHint,
    PermissionsRequired,
    SingleTouch,
    MultiTouch,
    Connected,
    Disconnected,
    PermissionDenied,
    Touchscreen,
    PenStylus,
    Mouse,
    Touchpad,
    NoProfiles,
    NoBackupFound,
    Delete,
    DeviceLabel,
    MethodLabelShort,
    SavedLabel,
    ApplySuccess,
    ResetSuccess,
    RestoreSuccess,
    DeviceDisconnectedMsg,
    EscToExit,
    UndoPoint,
    RetryPoint,
    LogViewer,
    AllLogsFilter,
}

pub fn raw_tr(lang: Language, key: TextKey) -> &'static str {
    match (lang, key) {
        (Language::English, TextKey::AppTitle) => "Smart Board Calibration",
        (Language::Persian, TextKey::AppTitle) => "کالیبراسیون تخته هوشمند و پروژکتور",

        (Language::English, TextKey::Subtitle) => {
            "Interactive Whiteboard & Projector Alignment Suite for Linux"
        }
        (Language::Persian, TextKey::Subtitle) => {
            "سامانه حرفه‌ای کالیبراسیون تخته هوشمند و پروژکتور تعاملی برای لینوکس"
        }

        (Language::English, TextKey::QuickCalibrate) => "Quick Calibrate (Fullscreen)",
        (Language::Persian, TextKey::QuickCalibrate) => "کالیبره سریع (تمام‌صفحه)",

        (Language::English, TextKey::QuickCalibrateHint) => {
            "One-click fullscreen target calibration on the selected display"
        }
        (Language::Persian, TextKey::QuickCalibrateHint) => {
            "اجرای فوری تمام‌صفحه برای لمس دونه به دونه نقاط روی صفحه"
        }

        (Language::English, TextKey::DetectHardware) => "Detect Hardware",
        (Language::Persian, TextKey::DetectHardware) => "شناسایی سخت‌افزار",

        (Language::English, TextKey::DetectHardwareDesc) => {
            "Scan monitors, projectors, and evdev touch controllers"
        }
        (Language::Persian, TextKey::DetectHardwareDesc) => {
            "اسکن نمایشگرها، پروژکتور و کنترلرهای لمسی سیستم"
        }

        (Language::English, TextKey::StartCalibration) => "Start Calibration",
        (Language::Persian, TextKey::StartCalibration) => "شروع کالیبراسیون",

        (Language::English, TextKey::StartCalibrationDesc) => {
            "Launch guided multi-point alignment wizard"
        }
        (Language::Persian, TextKey::StartCalibrationDesc) => {
            "اجرای ویزارد گام‌به‌گام کالیبراسیون دقیق تخته"
        }

        (Language::English, TextKey::TestTouch) => "Test Touch",
        (Language::Persian, TextKey::TestTouch) => "آزمایش لمس",

        (Language::English, TextKey::TestTouchDesc) => {
            "Verify raw coordinates, touch trail & pixel error on grid"
        }
        (Language::Persian, TextKey::TestTouchDesc) => {
            "بررسی مختصات خام، رسم مسیر لمس و خطای پیکسل روی شبکه"
        }

        (Language::English, TextKey::Profiles) => "Profiles",
        (Language::Persian, TextKey::Profiles) => "پروفایل‌ها",

        (Language::English, TextKey::ProfilesDesc) => {
            "Manage classroom profiles, startup auto-apply & backups"
        }
        (Language::Persian, TextKey::ProfilesDesc) => {
            "مدیریت پروفایل‌های کلاس، اجرای خودکار و بازگردانی نسخه قبلی"
        }

        (Language::English, TextKey::Settings) => "Settings",
        (Language::Persian, TextKey::Settings) => "تنظیمات",

        (Language::English, TextKey::SettingsDesc) => {
            "Tune target size, filters, language, theme & udev access"
        }
        (Language::Persian, TextKey::SettingsDesc) => {
            "تنظیم اندازه هدف، فیلتر نویز، زبان، تم و دسترسی‌های udev"
        }

        (Language::English, TextKey::Diagnostics) => "Diagnostics",
        (Language::Persian, TextKey::Diagnostics) => "عیب‌یابی و گزارش",

        (Language::English, TextKey::DiagnosticsDesc) => {
            "Inspect X11/Wayland, libinput, evdev permissions & logs"
        }
        (Language::Persian, TextKey::DiagnosticsDesc) => {
            "بررسی سرور نمایشگر، libinput، دسترسی‌ها و گزارش‌های سیستم"
        }

        (Language::English, TextKey::Home) => "Home",
        (Language::Persian, TextKey::Home) => "صفحه اصلی",

        (Language::English, TextKey::PreviousCalibrationFound) => "Previous calibration found for",
        (Language::Persian, TextKey::PreviousCalibrationFound) => "کالیبراسیون قبلی یافت شد برای",

        (Language::English, TextKey::Apply) => "Apply",
        (Language::Persian, TextKey::Apply) => "اعمال",

        (Language::English, TextKey::Recalibrate) => "Recalibrate",
        (Language::Persian, TextKey::Recalibrate) => "کالیبراسیون مجدد",

        (Language::English, TextKey::Dismiss) => "Dismiss",
        (Language::Persian, TextKey::Dismiss) => "بستن",

        (Language::English, TextKey::Step1Title) => "Step 1: Select Display",
        (Language::Persian, TextKey::Step1Title) => "مرحله ۱: انتخاب نمایشگر",

        (Language::English, TextKey::Step2Title) => "Step 2: Select Input Device",
        (Language::Persian, TextKey::Step2Title) => "مرحله ۲: انتخاب دستگاه ورودی",

        (Language::English, TextKey::Step3Title) => "Step 3: Select Calibration Type",
        (Language::Persian, TextKey::Step3Title) => "مرحله ۳: انتخاب نوع کالیبراسیون",

        (Language::English, TextKey::StepDisplayShort) => "1. Display",
        (Language::Persian, TextKey::StepDisplayShort) => "۱. نمایشگر",

        (Language::English, TextKey::StepDeviceShort) => "2. Input Device",
        (Language::Persian, TextKey::StepDeviceShort) => "۲. دستگاه لمسی",

        (Language::English, TextKey::StepMethodShort) => "3. Method & Start",
        (Language::Persian, TextKey::StepMethodShort) => "۳. روش و شروع",

        (Language::English, TextKey::Next) => "Next",
        (Language::Persian, TextKey::Next) => "مرحله بعد",

        (Language::English, TextKey::Back) => "Back",
        (Language::Persian, TextKey::Back) => "بازگشت",

        (Language::English, TextKey::Cancel) => "Cancel",
        (Language::Persian, TextKey::Cancel) => "انصراف",

        (Language::English, TextKey::Finish) => "Done",
        (Language::Persian, TextKey::Finish) => "پایان",

        (Language::English, TextKey::DisplaysHeading) => "Connected Displays",
        (Language::Persian, TextKey::DisplaysHeading) => "نمایشگرهای متصل",

        (Language::English, TextKey::InputDevicesHeading) => "Detected Input Devices",
        (Language::Persian, TextKey::InputDevicesHeading) => "دستگاه‌های ورودی شناسایی‌شده",

        (Language::English, TextKey::ProjectorBadge) => "Projector",
        (Language::Persian, TextKey::ProjectorBadge) => "پروژکتور",

        (Language::English, TextKey::PrimaryBadge) => "Primary",
        (Language::Persian, TextKey::PrimaryBadge) => "اصلی",

        (Language::English, TextKey::RecommendedBadge) => "Recommended",
        (Language::Persian, TextKey::RecommendedBadge) => "پیشنهادی",

        (Language::English, TextKey::ReadyBadge) => "Ready",
        (Language::Persian, TextKey::ReadyBadge) => "آماده",

        (Language::English, TextKey::SetupNeededBadge) => "udev rule needed",
        (Language::Persian, TextKey::SetupNeededBadge) => "نیاز به تنظیم udev",

        (Language::English, TextKey::TargetDisplayLabel) => "Target Display",
        (Language::Persian, TextKey::TargetDisplayLabel) => "نمایشگر هدف",

        (Language::English, TextKey::TargetDeviceLabel) => "Touch Input",
        (Language::Persian, TextKey::TargetDeviceLabel) => "ورودی لمسی",

        (Language::English, TextKey::SessionLabel) => "Display Server",
        (Language::Persian, TextKey::SessionLabel) => "سرور نمایشگر",

        (Language::English, TextKey::PermissionsLabel) => "Permissions",
        (Language::Persian, TextKey::PermissionsLabel) => "دسترسی‌ها",

        (Language::English, TextKey::ClickMonitorHint) => {
            "Click any display in the layout diagram below or choose from the list."
        }
        (Language::Persian, TextKey::ClickMonitorHint) => {
            "روی یکی از نمایشگرها در نقشه زیر کلیک کنید یا از فهرست انتخاب نمایید."
        }

        (Language::English, TextKey::LiveTouchSignalPrompt) => {
            "Tap your Smart Board now to verify live evdev signal from the selected device..."
        }
        (Language::Persian, TextKey::LiveTouchSignalPrompt) => {
            "برای اطمینان از درست بودن دستگاه، همین الان روی تخته هوشمند لمس کنید..."
        }

        (Language::English, TextKey::LiveTouchSignalReceived) => "Live touch signal detected! Raw",
        (Language::Persian, TextKey::LiveTouchSignalReceived) => {
            "سیگنال زنده لمس دریافت شد! مختصات خام"
        }

        (Language::English, TextKey::NoDevicesDetected) => {
            "No evdev touch or pointer devices detected."
        }
        (Language::Persian, TextKey::NoDevicesDetected) => {
            "هیچ دستگاه لمسی یا اشاره‌گر شناسایی نشد."
        }

        (Language::English, TextKey::PatternPreviewTitle) => "Calibration Target Grid Preview",
        (Language::Persian, TextKey::PatternPreviewTitle) => "پیش‌نمایش الگوی نقاط کالیبراسیون",

        (Language::English, TextKey::ProjectorMode) => {
            "Projector Mode (True Fullscreen & Sleep Lock)"
        }
        (Language::Persian, TextKey::ProjectorMode) => {
            "حالت پروژکتور (تمام‌صفحه واقعی و جلوگیری از خواب سیستم)"
        }

        (Language::English, TextKey::ProjectorModeHint) => {
            "Hides panels/taskbars, locks screen blanking & sleep, and restores system settings automatically on exit."
        }
        (Language::Persian, TextKey::ProjectorModeHint) => {
            "پنل‌ها و نوار وظیفه را پنهان کرده، مانع محافظ صفحه و خواب سیستم می‌شود و پس از پایان تنظیمات را بازمی‌گرداند."
        }

        (Language::English, TextKey::Cal4Point) => "4 Point (Fast Projective)",
        (Language::Persian, TextKey::Cal4Point) => "۴ نقطه‌ای (سریع)",

        (Language::English, TextKey::Cal9Point) => "9 Point (Recommended Default)",
        (Language::Persian, TextKey::Cal9Point) => "۹ نقطه‌ای (پیش‌فرض پیشنهادی)",

        (Language::English, TextKey::Cal16Point) => "16 Point (High Precision)",
        (Language::Persian, TextKey::Cal16Point) => "۱۶ نقطه‌ای (دقت بالا)",

        (Language::English, TextKey::Cal25Point) => "25 Point (Maximum Precision)",
        (Language::Persian, TextKey::Cal25Point) => "۲۵ نقطه‌ای (حداکثر دقت)",

        (Language::English, TextKey::CalAdvanced) => "Advanced Calibration (Nonlinear / Custom)",
        (Language::Persian, TextKey::CalAdvanced) => "کالیبراسیون پیشرفته (غیرخطی و سفارشی)",

        (Language::English, TextKey::DefaultBadge) => "Default",
        (Language::Persian, TextKey::DefaultBadge) => "پیش‌فرض",

        (Language::English, TextKey::MethodLabel) => "Transformation Model",
        (Language::Persian, TextKey::MethodLabel) => "مدل تبدیل مختصات",

        (Language::English, TextKey::MethodHomography) => "Projective Homography (DLT + LM)",
        (Language::Persian, TextKey::MethodHomography) => "هموگرافی تصویری (DLT + LM)",

        (Language::English, TextKey::MethodHomographyRansac) => "Robust Homography + RANSAC",
        (Language::Persian, TextKey::MethodHomographyRansac) => "هموگرافی مقاوم + RANSAC",

        (Language::English, TextKey::MethodAffine) => "Affine Least Squares (libinput)",
        (Language::Persian, TextKey::MethodAffine) => "آفین حداقل مربعات (libinput)",

        (Language::English, TextKey::MethodPoly2) => "Polynomial Degree 2",
        (Language::Persian, TextKey::MethodPoly2) => "چندجمله‌ای درجه ۲",

        (Language::English, TextKey::MethodPoly3) => "Polynomial Degree 3",
        (Language::Persian, TextKey::MethodPoly3) => "چندجمله‌ای درجه ۳",

        (Language::English, TextKey::MethodHybridTps) => "Hybrid Homography + Thin-Plate Spline",
        (Language::Persian, TextKey::MethodHybridTps) => "ترکیبی هموگرافی + TPS",

        (Language::English, TextKey::TouchTargetPrompt) => {
            "Touch and hold the center of the target"
        }
        (Language::Persian, TextKey::TouchTargetPrompt) => "مرکز هدف را لمس کرده و کوتاه نگه دارید",

        (Language::English, TextKey::CollectingSamples) => "Filtering touch samples...",
        (Language::Persian, TextKey::CollectingSamples) => "در حال نمونه‌برداری و حذف نویز...",

        (Language::English, TextKey::PointProgress) => "Target",
        (Language::Persian, TextKey::PointProgress) => "نقطه",

        (Language::English, TextKey::TouchInaccurateError) => {
            "Touch point was not detected accurately. Please touch the target again."
        }
        (Language::Persian, TextKey::TouchInaccurateError) => {
            "نقطه لمس به‌درستی تشخیص داده نشد. لطفاً دوباره روی هدف لمس کنید."
        }

        (Language::English, TextKey::CalibrationResults) => "Calibration Quality Evaluation",
        (Language::Persian, TextKey::CalibrationResults) => "ارزیابی کیفیت کالیبراسیون",

        (Language::English, TextKey::AccuracyVectorMapTitle) => {
            "Spatial Accuracy Map (Error vectors magnified ×15)"
        }
        (Language::Persian, TextKey::AccuracyVectorMapTitle) => {
            "نقشه مکانی دقت کالیبراسیون (بزرگ‌نمایی بردار خطا ۱۵×)"
        }

        (Language::English, TextKey::AccuracyMapLegend) => {
            "Green: <= 4 px | Amber: <= 12 px | Red: > 12 px / RANSAC outlier"
        }
        (Language::Persian, TextKey::AccuracyMapLegend) => {
            "سبز: خطای عالی | نارنجی: خطای متوسط | قرمز: خطای بالا یا Outlier"
        }

        (Language::English, TextKey::PointRowLabel) => "Point",
        (Language::Persian, TextKey::PointRowLabel) => "نقطه",

        (Language::English, TextKey::InliersLabel) => "RANSAC Inliers",
        (Language::Persian, TextKey::InliersLabel) => "نقاط معتبر RANSAC",

        (Language::English, TextKey::ExpectedPos) => "Expected",
        (Language::Persian, TextKey::ExpectedPos) => "مختصات مورد انتظار",

        (Language::English, TextKey::ActualPos) => "Calibrated",
        (Language::Persian, TextKey::ActualPos) => "مختصات محاسبه‌شده",

        (Language::English, TextKey::PixelError) => "Pixel Error",
        (Language::Persian, TextKey::PixelError) => "خطای پیکسل",

        (Language::English, TextKey::AverageError) => "Average Error",
        (Language::Persian, TextKey::AverageError) => "میانگین خطا",

        (Language::English, TextKey::MaximumError) => "Maximum Error",
        (Language::Persian, TextKey::MaximumError) => "حداکثر خطا",

        (Language::English, TextKey::RmsError) => "RMS Error",
        (Language::Persian, TextKey::RmsError) => "خطای RMS",

        (Language::English, TextKey::CalibrationQualityLabel) => "Calibration Quality",
        (Language::Persian, TextKey::CalibrationQualityLabel) => "کیفیت کالیبراسیون",

        (Language::English, TextKey::QualityExcellent) => "Excellent",
        (Language::Persian, TextKey::QualityExcellent) => "عالی",

        (Language::English, TextKey::QualityGood) => "Good",
        (Language::Persian, TextKey::QualityGood) => "خوب",

        (Language::English, TextKey::QualityAcceptable) => "Acceptable",
        (Language::Persian, TextKey::QualityAcceptable) => "قابل قبول",

        (Language::English, TextKey::QualityPoor) => "Poor",
        (Language::Persian, TextKey::QualityPoor) => "ضعیف",

        (Language::English, TextKey::QualityInsufficient) => {
            "Calibration quality is insufficient. Recalibrate?"
        }
        (Language::Persian, TextKey::QualityInsufficient) => {
            "کیفیت کالیبراسیون کافی نیست. آیا مایل به کالیبراسیون مجدد هستید؟"
        }

        (Language::English, TextKey::ProfileNameLabel) => "Profile Name",
        (Language::Persian, TextKey::ProfileNameLabel) => "نام پروفایل",

        (Language::English, TextKey::ApplyCalibration) => "Apply Calibration",
        (Language::Persian, TextKey::ApplyCalibration) => "اعمال کالیبراسیون",

        (Language::English, TextKey::ApplyOnStartup) => "Apply automatically on startup",
        (Language::Persian, TextKey::ApplyOnStartup) => "اعمال خودکار هنگام راه‌اندازی سیستم",

        (Language::English, TextKey::SaveProfile) => "Save Profile",
        (Language::Persian, TextKey::SaveProfile) => "ذخیره پروفایل",

        (Language::English, TextKey::ResetCalibration) => "Reset Calibration",
        (Language::Persian, TextKey::ResetCalibration) => "بازنشانی کالیبراسیون",

        (Language::English, TextKey::RestorePreviousCalibration) => "Restore Previous Calibration",
        (Language::Persian, TextKey::RestorePreviousCalibration) => "بازیابی کالیبراسیون قبلی",

        (Language::English, TextKey::TestModeHeading) => "Interactive Touch Verification Grid",
        (Language::Persian, TextKey::TestModeHeading) => "حالت آزمایش و بررسی دقت لمس",

        (Language::English, TextKey::TestModeHint) => {
            "Touch or draw across the grid to inspect raw coordinates, calibrated screen position, and pixel error."
        }
        (Language::Persian, TextKey::TestModeHint) => {
            "روی شبکه لمس کنید یا مسیر بکشید تا مختصات خام، مختصات صفحه و میزان خطا را بررسی کنید."
        }

        (Language::English, TextKey::ClearPath) => "Clear Path",
        (Language::Persian, TextKey::ClearPath) => "پاک کردن مسیر",

        (Language::English, TextKey::ExitTestMode) => "Exit Test Mode",
        (Language::Persian, TextKey::ExitTestMode) => "خروج از حالت آزمایش",

        (Language::English, TextKey::WindowedLabel) => "Windowed",
        (Language::Persian, TextKey::WindowedLabel) => "حالت پنجره‌ای",

        (Language::English, TextKey::FullscreenLabel) => "Fullscreen",
        (Language::Persian, TextKey::FullscreenLabel) => "تمام‌صفحه",

        (Language::English, TextKey::GridDensityLabel) => "Grid",
        (Language::Persian, TextKey::GridDensityLabel) => "شبکه",

        (Language::English, TextKey::RawCoords) => "Raw",
        (Language::Persian, TextKey::RawCoords) => "مختصات خام",

        (Language::English, TextKey::ScreenCoords) => "Screen",
        (Language::Persian, TextKey::ScreenCoords) => "مختصات صفحه",

        (Language::English, TextKey::ErrorCoords) => "Error",
        (Language::Persian, TextKey::ErrorCoords) => "خطا",

        (Language::English, TextKey::SettingsTargetSize) => "Target Size (px)",
        (Language::Persian, TextKey::SettingsTargetSize) => "اندازه هدف (پیکسل)",

        (Language::English, TextKey::SettingsHighContrast) => "High Contrast Canvas",
        (Language::Persian, TextKey::SettingsHighContrast) => "کنتراست بالا",

        (Language::English, TextKey::SettingsTargetDwell) => "Target Timeout (s)",
        (Language::Persian, TextKey::SettingsTargetDwell) => "مدت انتظار هدف (ثانیه)",

        (Language::English, TextKey::SettingsSampleCount) => "Samples per Target",
        (Language::Persian, TextKey::SettingsSampleCount) => "تعداد نمونه در هر نقطه",

        (Language::English, TextKey::SettingsMinDuration) => "Min Touch Duration (ms)",
        (Language::Persian, TextKey::SettingsMinDuration) => "حداقل زمان لمس (میلی‌ثانیه)",

        (Language::English, TextKey::SettingsMaxDeviation) => "Max Deviation Threshold",
        (Language::Persian, TextKey::SettingsMaxDeviation) => "حداکثر انحراف مجاز",

        (Language::English, TextKey::SettingsDiscardInitial) => "Initial Samples Discarded",
        (Language::Persian, TextKey::SettingsDiscardInitial) => "حذف نمونه‌های اولیه",

        (Language::English, TextKey::SettingsTouchSensitivity) => "Touch Sensitivity",
        (Language::Persian, TextKey::SettingsTouchSensitivity) => "حساسیت لمس",

        (Language::English, TextKey::SettingsLanguage) => "Language",
        (Language::Persian, TextKey::SettingsLanguage) => "زبان برنامه",

        (Language::English, TextKey::SettingsTheme) => "Appearance",
        (Language::Persian, TextKey::SettingsTheme) => "ظاهر برنامه",

        (Language::English, TextKey::LightMode) => "Light",
        (Language::Persian, TextKey::LightMode) => "روشن",

        (Language::English, TextKey::DarkMode) => "Dark",
        (Language::Persian, TextKey::DarkMode) => "تیره",

        (Language::English, TextKey::ExportDiagnostics) => "Export Diagnostics",
        (Language::Persian, TextKey::ExportDiagnostics) => "خروجی گزارش عیب‌یابی",

        (Language::English, TextKey::OpenLogs) => "Open Logs",
        (Language::Persian, TextKey::OpenLogs) => "باز کردن لاگ‌ها",

        (Language::English, TextKey::InstallUdevRule) => "Install udev Permissions Rule",
        (Language::Persian, TextKey::InstallUdevRule) => "نصب قانون دسترسی udev",

        (Language::English, TextKey::UdevSectionTitle) => "Linux Non-Root Input Permissions (udev)",
        (Language::Persian, TextKey::UdevSectionTitle) => {
            "دسترسی ورودی بدون نیاز به Root در لینوکس (udev)"
        }

        (Language::English, TextKey::UdevInstallSuccess) => "udev rule installed successfully.",
        (Language::Persian, TextKey::UdevInstallSuccess) => "قانون udev با موفقیت نصب شد.",

        (Language::English, TextKey::UdevInstallHint) => {
            "Run in terminal: sudo smartcal --install-udev-rule"
        }
        (Language::Persian, TextKey::UdevInstallHint) => {
            "در ترمینال اجرا کنید: sudo smartcal --install-udev-rule"
        }

        (Language::English, TextKey::PermissionsRequired) => {
            "Additional system permissions are required to apply this calibration."
        }
        (Language::Persian, TextKey::PermissionsRequired) => {
            "برای اعمال این کالیبراسیون به دسترسی‌های بیشتر سیستم نیاز است."
        }

        (Language::English, TextKey::SingleTouch) => "Single Touch",
        (Language::Persian, TextKey::SingleTouch) => "تک‌لمسی",

        (Language::English, TextKey::MultiTouch) => "Multi Touch",
        (Language::Persian, TextKey::MultiTouch) => "چندلمسی",

        (Language::English, TextKey::Connected) => "Connected",
        (Language::Persian, TextKey::Connected) => "متصل",

        (Language::English, TextKey::Disconnected) => "Disconnected",
        (Language::Persian, TextKey::Disconnected) => "قطع شده",

        (Language::English, TextKey::PermissionDenied) => "Permission Denied",
        (Language::Persian, TextKey::PermissionDenied) => "عدم دسترسی",

        (Language::English, TextKey::Touchscreen) => "Touchscreen",
        (Language::Persian, TextKey::Touchscreen) => "صفحه لمسی",

        (Language::English, TextKey::PenStylus) => "Pen / Stylus",
        (Language::Persian, TextKey::PenStylus) => "قلم نوری",

        (Language::English, TextKey::Mouse) => "Mouse",
        (Language::Persian, TextKey::Mouse) => "ماوس",

        (Language::English, TextKey::Touchpad) => "Touchpad",
        (Language::Persian, TextKey::Touchpad) => "تاچ‌پد",

        (Language::English, TextKey::NoProfiles) => "No calibration profiles saved yet.",
        (Language::Persian, TextKey::NoProfiles) => "هنوز هیچ پروفایل کالیبراسیونی ذخیره نشده است.",

        (Language::English, TextKey::NoBackupFound) => "No previous calibration backup available.",
        (Language::Persian, TextKey::NoBackupFound) => "پشتیبان کالیبراسیون قبلی موجود نیست.",

        (Language::English, TextKey::Delete) => "Delete",
        (Language::Persian, TextKey::Delete) => "حذف",

        (Language::English, TextKey::DeviceLabel) => "Device",
        (Language::Persian, TextKey::DeviceLabel) => "دستگاه",

        (Language::English, TextKey::MethodLabelShort) => "Method",
        (Language::Persian, TextKey::MethodLabelShort) => "مدل",

        (Language::English, TextKey::SavedLabel) => "Saved",
        (Language::Persian, TextKey::SavedLabel) => "زمان ذخیره",

        (Language::English, TextKey::ApplySuccess) => "Calibration applied successfully.",
        (Language::Persian, TextKey::ApplySuccess) => "کالیبراسیون با موفقیت اعمال شد.",

        (Language::English, TextKey::ResetSuccess) => "Calibration reset to factory identity.",
        (Language::Persian, TextKey::ResetSuccess) => {
            "کالیبراسیون به حالت پیش‌فرض کارخانه بازنشانی شد."
        }

        (Language::English, TextKey::RestoreSuccess) => "Previous calibration restored.",
        (Language::Persian, TextKey::RestoreSuccess) => "کالیبراسیون قبلی با موفقیت بازیابی شد.",

        (Language::English, TextKey::DeviceDisconnectedMsg) => {
            "Input device disconnected. Waiting for USB reconnection..."
        }
        (Language::Persian, TextKey::DeviceDisconnectedMsg) => {
            "اتصال دستگاه ورودی قطع شد. در انتظار اتصال مجدد USB..."
        }

        (Language::English, TextKey::EscToExit) => "Press ESC to cancel and restore system state",
        (Language::Persian, TextKey::EscToExit) => {
            "برای لغو و بازگرداندن تنظیمات سیستم کلید ESC را بزنید"
        }

        (Language::English, TextKey::UndoPoint) => "Undo Point",
        (Language::Persian, TextKey::UndoPoint) => "برگشت نقطه قبل",

        (Language::English, TextKey::RetryPoint) => "Retry Point",
        (Language::Persian, TextKey::RetryPoint) => "تکرار نقطه",

        (Language::English, TextKey::LogViewer) => "System Logs",
        (Language::Persian, TextKey::LogViewer) => "گزارش رویدادها",

        (Language::English, TextKey::AllLogsFilter) => "ALL",
        (Language::Persian, TextKey::AllLogsFilter) => "همه",
    }
}

/// Returns logical UTF-8 translation (egui 0.37 / harfrust shapes Persian natively via OpenType).
pub fn tr(lang: Language, key: TextKey) -> String {
    raw_tr(lang, key).to_string()
}

/// Returns clean logical UTF-8 text (visual BiDi splitting is handled by LayoutJob in GUI).
pub fn shape_text(_lang: Language, text: &str) -> String {
    text.to_string()
}

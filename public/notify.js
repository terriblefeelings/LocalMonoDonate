/* Functional controller for /notify. Presentation was removed from the page; this file contains behavior and DOM contracts only. */
const $ = id => document.getElementById(id);
const DEFAULTS = Alert.DEFAULTS;
const STYLE_KEYS = ['top_text','money_text','blue_font','blue_size','blue_weight','blue_color','blue_outline','blue_outline_op','blue_maxw','blue_tracking','blue_line','green_font','green_size','green_weight','green_color','green_outline','green_outline_op','green_maxw','green_tracking','green_line','msg_font','msg_size','msg_weight','msg_color','msg_outline','msg_outline_op','msg_maxw','msg_maxh','win_bg','win_bg_opacity','win_radius','win_maxh','win_border_color','win_border_width','win_shadow','win_padding','blue_bg','blue_bg_opacity','blue_radius','green_bg','green_bg_opacity','green_radius','red_bg','red_bg_opacity','red_radius','duration_ms','wait_media','anim_in','anim_out','anim_ms','image_url','video_url','sound_url','media_pos','media_w','media_h','media_zoom','media_radius','media_pan_x','media_pan_y','max_width'];
const NUM = new Set(['blue_size','blue_weight','blue_outline_op','blue_maxw','blue_tracking','blue_line','green_size','green_weight','green_outline_op','green_maxw','green_tracking','green_line','msg_size','msg_weight','msg_outline_op','msg_maxw','msg_maxh','win_bg_opacity','win_radius','win_maxh','win_border_width','win_shadow','win_padding','blue_bg_opacity','blue_radius','green_bg_opacity','green_radius','red_bg_opacity','red_radius','duration_ms','anim_ms','media_w','media_h','media_zoom','media_radius','media_pan_x','media_pan_y','max_width']);
const BOOL = new Set(['wait_media']);
const DICT = {
  en:{save:'Save',variation:'Donation variation',rulesHint:'the first matching rule wins; the default one is the fallback',addRule:'+ Add variation',from:'from',toInf:'to',allVoices:'All',ttsMode:'Voice set',tmAzuk:'Azure UA (Polina/Ostap)',tmAzus:'Azure US (Jenny/Guy)',azKey:'Azure Speech key',azRegion:'Azure region',azWarn:'Fill in the Azure key and region below to use this voice set.',azHelp:'How to get an Azure key (free, 500k characters/month): 1) sign in at portal.azure.com; 2) Create a resource, search Speech; 3) pick a resource group + a Region (remember it, e.g. westeurope); 4) Pricing tier = Free F0; 5) Create, then open the resource, Keys and Endpoint, copy KEY 1 and the Location. Paste them above and Save.',tmDefault:'Default (all voices)',tmGuk:'Google UA only',tmGus:'Google US only',tmCustom:'Custom only',tmOff:'Off (no voice)',defVoice:'Default voice',readMode:'Read aloud',ttsSpeed:'Speech speed',rmAnn:'Announcement only',rmMsg:'Message only',rmBoth:'Announcement + message',ttsNote:'Applies to all variations. Choose Google UA/US, or Azure after entering a key.',gText:'Text',lineBlue:'Top line (light blue)',lineGreen:'Amount line (green)',varsHint:'%donate% = user name, %money% = amount; both work in either line',gBlue:'Top line style',gGreen:'Amount line style',gMsg:'Message',gWindow:'Window background',font:'Font',size:'Size (px)',weight:'Weight',color:'Text colour',outline:'Outline colour',outlineOp:'Outline opacity',maxw:'Max width (px)',maxh:'Max height (px)',bgc:'Background',bgo:'Background opacity',bgr:'Background radius (px)',winMaxH:'Max height (px, 0 = auto)',gDuration:'Duration',showMs:'Show for (ms, image/none)',keepMedia:'Hold while audio/video plays',gAnim:'Animation',animIn:'In',animOut:'Out',speedMs:'Speed (ms)',aUp:'slide up',aFade:'fade',aZoom:'zoom',aLeft:'slide left',aDown:'slide down',gMedia:'Media & sound',image:'Image (png/jpg/gif…)',video:'Video (mp4/webm…)',sound:'Sound (mp3/…)',mPos:'Position vs window',mpTop:'top',mpBottom:'bottom',mpLeft:'left',mpRight:'right',mSize:'Size (px W×H)',mZoom:'Zoom (%)',mRadius:'Radius (px)',mediaHint:'drag inside the media to reposition it (pan) · zoom & size above',gExport:'Export / import',exportJson:'Export JSON',importJson:'Import JSON',reset:'Reset',defaultLabel:'Default (rest)',saved:'saved — server running',name:'Name',confirmReset:'Reset this variation to default settings?',resetDone:'variation reset to defaults'},
  ua:{save:'Зберегти',variation:'Варіація донату',rulesHint:'спрацьовує перше правило, що збігається; за замовчуванням — запасне',addRule:'+ Додати варіацію',from:'від',toInf:'до',allVoices:'Усі',ttsMode:'Набір голосів',tmAzuk:'Azure UA (Поліна/Остап)',tmAzus:'Azure US (Дженні/Гай)',azKey:'Ключ Azure Speech',azRegion:'Регіон Azure',azWarn:'Заповніть ключ і регіон Azure нижче, щоб використати цей набір голосів.',azHelp:'Як отримати ключ Azure (безкоштовно, 500 тис. символів на місяць): 1) увійдіть на portal.azure.com; 2) «Створити ресурс» (Create a resource), знайдіть Speech; 3) виберіть групу ресурсів і регіон (Region), наприклад westeurope; 4) рівень цін (Pricing tier) — Free F0; 5) «Створити» (Create), відкрийте ресурс, «Ключі та кінцева точка» (Keys and Endpoint), скопіюйте KEY 1 і Location. Вставте їх вище та натисніть «Зберегти».',tmDefault:'За замовчуванням (усі голоси)',tmGuk:'Лише Google UA',tmGus:'Лише Google US',tmCustom:'Лише власні',tmOff:'Вимкнено (без голосу)',defVoice:'Голос за замовчуванням',readMode:'Озвучувати',ttsSpeed:'Швидкість мовлення',rmAnn:'Лише оголошення',rmMsg:'Лише повідомлення',rmBoth:'Оголошення + повідомлення',ttsNote:'Застосовується до всіх варіацій. Обери Google UA/US або Azure після введення ключа.',gText:'Текст',lineBlue:'Верхній рядок (блакитний)',lineGreen:'Рядок суми (зелений)',varsHint:'%donate% = імʼя, %money% = сума; працюють в обох рядках',gBlue:'Стиль верхнього рядка',gGreen:'Стиль рядка суми',gMsg:'Повідомлення',gWindow:'Фон вікна',font:'Шрифт',size:'Розмір (px)',weight:'Товщина',color:'Колір тексту',outline:'Колір обрису',outlineOp:'Прозорість обрису',maxw:'Макс. ширина (px)',maxh:'Макс. висота (px)',bgc:'Фон',bgo:'Прозорість фону',bgr:'Радіус фону (px)',winMaxH:'Макс. висота вікна (px, 0 = авто)',gDuration:'Тривалість',showMs:'Показ (мс, зображення/немає)',keepMedia:'Тримати, поки аудіо/відео грає',gAnim:'Анімація',animIn:'Поява',animOut:'Зникнення',speedMs:'Швидкість (мс)',aUp:'знизу',aFade:'прозоро',aZoom:'зум',aLeft:'зліва',aDown:'вниз',gMedia:'Медіа та звук',image:'Зображення (png/jpg/gif…)',video:'Відео (mp4/webm…)',sound:'Звук (mp3/…)',mPos:'Позиція відносно вікна',mpTop:'зверху',mpBottom:'знизу',mpLeft:'зліва',mpRight:'справа',mSize:'Розмір (px Ш×В)',mZoom:'Зум (%)',mRadius:'Радіус (px)',mediaHint:'тягни всередині медіа щоб зсунути · зум і розмір вище',gExport:'Експорт / імпорт',exportJson:'Експорт JSON',importJson:'Імпорт JSON',reset:'Скинути',defaultLabel:'За замовчуванням (решта)',saved:'збережено ✓',name:'Імʼя',confirmReset:'Скинути цю варіацію до стандартних налаштувань?',resetDone:'варіацію скинуто до стандартних'}
};
window.I18N = { en: DICT.en, ua: DICT.ua };
Object.assign(DICT.en, {
  consoleSystem:'Donation Alerts', consoleReady:'Preview', photoContext:'Studio mixer',
  consoleTitle:'Alert setup', consoleIntro:'Tune how donation alerts look and sound, then preview the result.',
  consoleProfile:'Editing', cueBank:'Rules', cueStack:'Variations', rulesHint:'The first matching rule wins. The default one is the fallback.',
  cueSingular:'variation', cuePlural:'variations', poolLabel:'Add to default pool', triggerRange:'Amount range', currencyUnit:'UAH', monitorChannel:'Stream preview',
  monitorTitle:'Alert Preview', previewOnly:'Preview', previewLocal:'Only you can see this preview; it is not broadcast.', previewDrag:'Drag media to move it',
  settingsIndex:'Fine-tune', settingsTitle:'Cue settings', settingsHint:'Control its sound, motion, text, and media.',
  settingsHintRight:'Changes apply to the selected variation', fixedCueNote:'Fixed amount — this cue plays only for the configured range.',
  poolCueNote:'Pool cue — this variation shares equal selection chance with the default.',
  testVariation:'Test this variation', deleteVariation:'Delete variation', selectVariation:'Select variation', previewDonor:'Sample user', mediaTest:'Media test', testMD:'Test MD', testYT:'Test YT', testMDTitle:'Test with media link', testYTTitle:'Test with YouTube', testMDLink:'Media link for Test MD', testMDNote:'Any supported YouTube, Instagram, TikTok, Twitch, Reddit or X link. Used only by Test MD.', testRuleLabel:'Test variation',
  specSignal:'Signal / local', specOutput:'No live output', specFeed:'Local feed', photoCaption:'Studio reference', photoSource:'Console detail', photoRef:'REF. 01'
});
Object.assign(DICT.ua, {
  consoleSystem:'Сповіщення про донати', consoleReady:'Попередній перегляд', photoContext:'Мікшерна студія',
  consoleTitle:'Алерти', consoleIntro:'Налаштуйте вигляд і звук сповіщення та перегляньте результат.',
  consoleProfile:'Редагується', cueBank:'Правила', cueStack:'Варіації', rulesHint:'Спрацьовує перше правило, що збігається. За замовчуванням — запасне.',
  cueSingular:'варіація', cuePlural:'варіації', poolLabel:'Додати до пулу за замовчуванням', triggerRange:'Діапазон суми', currencyUnit:'грн', monitorChannel:'Перегляд для стріму',
  monitorTitle:'Попередній перегляд', previewOnly:'Превʼю', previewLocal:'Цей перегляд бачите лише ви; він не транслюється.', previewDrag:'Перетягніть медіа, щоб перемістити',
  settingsIndex:'Детальне налаштування', settingsTitle:'Параметри алерта', settingsHint:'Налаштуйте звук, рух, текст і медіа.',
  settingsHintRight:'Зміни стосуються вибраної варіації', fixedCueNote:'Фіксована сума — правило діє лише для заданого діапазону.',
  poolCueNote:'Спільний пул — це правило має однаковий шанс з іншими правилами пулу.',
  testVariation:'Перевірити цю варіацію', deleteVariation:'Видалити варіацію', selectVariation:'Вибрати варіацію', previewDonor:'Тестовий користувач', mediaTest:'Тест медіа', testMD:'Тест MD', testYT:'Тест YT', testMDTitle:'Тест із посиланням на медіа', testYTTitle:'Тест із YouTube', testMDLink:'Посилання на медіа для тесту MD', testMDNote:'Будь-яке підтримуване посилання YouTube, Instagram, TikTok, Twitch, Reddit або X. Використовується лише для тесту MD.', testRuleLabel:'Тестова варіація',
  specSignal:'Сигнал / локально', specOutput:'Без ефіру', specFeed:'Локальний сигнал', photoCaption:'Студійний референс', photoSource:'Деталь консолі', photoRef:'РЕФ. 01'
});
Object.assign(DICT.en, {
  consoleTitle:'Alert Configuration',
  consoleIntro:'Shape the signal, voice, and framing of each donation alert. Every change renders in the local preview.',
  cueBank:'Cue Bank', cueBankHint:'Select a cue to inspect its range and styling.',
  outputLabel:'Output Monitor', parameterConsole:'Parameter Console',
  signalCopy:'Cue copy', signalCopyType:'Copy and type', signalVoice:'Voice channel', signalTiming:'Timing and motion',
  signalMessage:'Message typography', signalTop:'Top line', signalAmount:'Amount line',
  signalFrame:'Output frame', signalMedia:'Media channel', signalData:'Configuration data',
  testShort:'Test', poolShort:'pool', upload:'Load', centerMedia:'Center',
  localOnly:'Local only', statusLocal:'Local preview', saveHint:'Save from the top bar', poolRemove:'Remove from default pool',
  azHelpTitle:'Azure key setup'
});
Object.assign(DICT.ua, {
  consoleTitle:'Конфігурація алертів',
  consoleIntro:'Налаштуйте сигнал, голос і рамку кожного алерта. Кожна зміна одразу відображається в локальному перегляді.',
  cueBank:'Банк правил', cueBankHint:'Виберіть правило, щоб переглянути його діапазон і стиль.',
  outputLabel:'Монітор виходу', parameterConsole:'Консоль параметрів',
  signalCopy:'Текст сигналу', signalCopyType:'Текст і типографіка', signalVoice:'Голосовий канал', signalTiming:'Час і рух',
  signalMessage:'Типографіка повідомлення', signalTop:'Верхній рядок', signalAmount:'Рядок суми',
  signalFrame:'Рамка виходу', signalMedia:'Медіаканал', signalData:'Дані конфігурації',
  testShort:'Тест', poolShort:'пул', upload:'Завантажити', centerMedia:'Центрувати',
  localOnly:'Лише локально', statusLocal:'Локальний перегляд', saveHint:'Збереження — у верхній панелі', poolRemove:'Прибрати з пулу за замовчуванням',
  azHelpTitle:'Налаштування ключа Azure'
});
/* Copy pass: natural bilingual wording, not a literal swap. */
Object.assign(DICT.en, {
  save:'Save', defaultLabel:'Default (rest)', name:'Variation',
  consoleTitle:'Alerts Configuration', titleLead:'Alerts', titleAccent:'Configuration', consoleIntro:'Tune how each donation alert looks and sounds, then check the result in the local preview.',
  consoleSystem:'Donation Alerts', statusLocal:'Local preview', cueBank:'Cue Bank', cueStack:'Variations',
  cueBankHint:'Pick a cue to review its range and styling.', rulesHint:'The first matching rule wins; the default is the fallback.',
  addRule:'+ Add variation', triggerRange:'Amount range', currencyUnit:'UAH', from:'from', toInf:'to',
  cueSingular:'variation', cuePlural:'variations', variationName:'Variation name',
  fixedCueNote:'Fixed amount — this cue plays only in this range.', poolCueNote:'Pool cue — same chance as the default.',
  testShort:'Test', poolShort:'pool', addedToPool:'Added to pool', poolLabel:'Add to pool', poolRemove:'Remove from pool',
  testVariation:'Test this variation', deleteVariation:'Delete variation', selectVariation:'Select variation',
  outputLabel:'Output Monitor', monitorTitle:'Alert Preview', previewOnly:'Preview', previewLocal:'Local preview only — nothing is broadcast.',
  previewDrag:'Scroll to zoom; drag to pan; double-click to center.', hoverZoomNote:'Scroll over the preview to change media zoom. Drag to pan; double-click to recenter.',
  parameterConsole:'Parameter Console', settingsHint:'Tune the voice, motion, text, media, and frame.', settingsHintRight:'Applies to the selected cue',
  signalCopyType:'Copy and type', signalTop:'Top line', signalAmount:'Amount line', signalVoice:'Voice', signalTiming:'Timing and motion',
  signalFrame:'Output frame', signalMedia:'Media channel', signalData:'Configuration data',
  lineBlue:'Top line', lineGreen:'Amount line', varsHint:'%donate% = user, %money% = amount. Both work in either line.',
  lineNote:'Tracking and line height apply to the broadcast alert.', tracking:'Tracking (px)', lineHeight:'Line height',
  winBorder:'Border colour', winBorderW:'Border width (px)', winShadow:'Shadow strength', winPadding:'Inner padding (px)',
  frameNote:'Border, shadow, and padding affect the alert window only.',
  saveHint:'Save in the top bar', aRight:'slide right', aRise:'rise', aFlip:'flip',
  themeDark:'Dark theme', themeLight:'Light theme', langEn:'English', langUa:'Ukrainian',
  saved:'saved — server running', resetDone:'Cue reset to defaults.', confirmReset:'Reset this cue?',
  upload:'Load', centerMedia:'Center', exportJson:'Export JSON', importJson:'Import JSON', reset:'Reset',
  mSize:'Size W×H (px)', mZoom:'Zoom (%)', mRadius:'Radius (px)', mediaHint:'Scroll over the preview to zoom; drag to pan; double-click to center.',
  keepMedia:'Hold while audio/video plays', showMs:'Show for (ms; image/none)', winMaxH:'Window max height (px; 0 = auto)',
  ruleNameNote:'Shown in the cue bank. The default cue cannot be renamed.',
  ruleMinNote:'Lower amount bound for this cue.', ruleMaxNote:'Upper amount bound; 0 means unlimited.', addRuleNote:'Add a new cue before the default fallback.',
  topTextNote:'Top-line template. %donate% is the user, %money% is the amount.', moneyTextNote:'Amount-line template. Both variables work in either line.',
  sizeNote:'Text or box size in pixels.', weightNote:'Font weight; higher values are bolder.', colorNote:'Colour used by this element.',
  opacityNote:'0 is transparent, 1 is fully visible.', radiusNote:'Corner rounding in pixels.', winBgNote:'Background colour of the alert window.',
  borderNote:'Colour and thickness of the alert window border.', shadowNote:'Outer shadow strength; 0 removes the shadow.', paddingNote:'Space between the window border and its content.',
  durationNote:'How long the alert stays on screen.', animInNote:'How the alert enters the screen.', animOutNote:'How the alert leaves the screen.', speedNote:'Animation duration in milliseconds.',
  mediaUrlNote:'Local or remote media URL. Load opens a file picker.', uploadNote:'Upload a local media file.', mediaPosNote:'Where the media sits relative to the alert window.',
  mediaSizeNote:'Media box width and height in pixels.', mediaZoomNote:'Media zoom percentage. Scroll over the preview to adjust it.', centerNote:'Reset the media pan to the centre.',
  exportNote:'Download the current cue set as JSON.', importNote:'Load a cue set from a JSON file.', resetNote:'Reset the selected cue to the built-in defaults.',
  previewNote:'Scroll over the preview to zoom media; drag to pan; double-click to centre.'
});
Object.assign(DICT.ua, {
  save:'Зберегти', defaultLabel:'За замовчуванням (решта)', name:'Варіант',
  consoleTitle:'Налаштування алертів', titleLead:'Налаштування', titleAccent:'алертів', consoleIntro:'Налаштуйте вигляд і звучання кожного алерта та перевірте результат у локальному перегляді.',
  consoleSystem:'Сповіщення про донати', statusLocal:'Локальний перегляд', cueBank:'Банк алертів', cueStack:'Варіанти',
  cueBankHint:'Виберіть алерт, щоб переглянути його діапазон і стиль.', rulesHint:'Спрацьовує перший збіг; правило за замовчуванням — запасне.',
  addRule:'+ Додати варіант', triggerRange:'Діапазон суми', currencyUnit:'грн', from:'від', toInf:'до',
  cueSingular:'варіант', cuePlural:'варіантів', cueFew:'варіанти', variationName:'Назва варіанту',
  fixedCueNote:'Фіксована сума — цей алерт працює лише в цьому діапазоні.', poolCueNote:'Спільний пул — рівний шанс з іншими правилами пулу.',
  testShort:'ТЕСТ', poolShort:'пул', addedToPool:'Додано до пулу', poolLabel:'Додати до пулу', poolRemove:'Прибрати з пулу',
  testVariation:'Перевірити цей варіант', deleteVariation:'Видалити варіант', selectVariation:'Вибрати варіант',
  outputLabel:'Монітор виводу', monitorTitle:'Перегляд алерта', previewOnly:'Прев’ю', previewLocal:'Лише локальний перегляд — нічого не транслюється.',
  previewDrag:'Прокрутіть, щоб наблизити; перетягніть, щоб змістити; двічі клацніть, щоб центрувати.', hoverZoomNote:'Прокрутіть над переглядом, щоб змінити масштаб медіа. Перетягніть, щоб змістити; двічі клацніть, щоб центрувати.',
  parameterConsole:'Параметри', settingsHint:'Налаштуйте голос, рух, текст, медіа та рамку.', settingsHintRight:'Застосовується до вибраного алерта',
  signalCopyType:'Текст і шрифт', signalTop:'Верхній рядок', signalAmount:'Рядок суми', signalVoice:'Голос', signalTiming:'Час і рух',
  signalFrame:'Рамка алерта', signalMedia:'Медіа', signalData:'Дані',
  lineBlue:'Верхній рядок', lineGreen:'Рядок суми', varsHint:'%donate% — ім’я користувача, %money% — сума. Працюють в обох рядках.',
  lineNote:'Міжлітерний інтервал і висота рядка діють на ефірний алерт.', tracking:'Міжлітерний інтервал (px)', lineHeight:'Висота рядка',
  winBorder:'Колір рамки', winBorderW:'Товщина рамки (px)', winShadow:'Сила тіні', winPadding:'Внутрішній відступ (px)',
  frameNote:'Рамка, тінь і відступи впливають лише на вікно алерта.',
  saveHint:'Збереження — у верхній панелі', aRight:'справа', aRise:'підйом', aFlip:'переворот',
  themeDark:'Темна тема', themeLight:'Світла тема', langEn:'Англійська', langUa:'Українська',
  saved:'збережено — сервер працює', resetDone:'Алерт скинуто до налаштувань за замовчуванням.', confirmReset:'Скинути цей алерт?',
  upload:'Завантажити', centerMedia:'Центрувати', exportJson:'Експорт JSON', importJson:'Імпорт JSON', reset:'Скинути',
  mSize:'Розмір Ш×В (px)', mZoom:'Масштаб (%)', mRadius:'Радіус (px)', mediaHint:'Прокрутіть над переглядом, щоб наблизити; перетягніть, щоб змістити; двічі клацніть, щоб центрувати.',
  keepMedia:'Тримати, поки грає аудіо/відео', showMs:'Показ (мс; зображення/немає)', winMaxH:'Макс. висота вікна (px; 0 = авто)',
  ruleNameNote:'Показується в банку алертів. Алерт за замовчуванням перейменувати не можна.',
  ruleMinNote:'Нижня межа суми для цього алерта.', ruleMaxNote:'Верхня межа суми; 0 — без обмеження.', addRuleNote:'Додати новий алерт перед запасним варіантом за замовчуванням.',
  topTextNote:'Шаблон верхнього рядка. %donate% — користувач, %money% — сума.', moneyTextNote:'Шаблон рядка суми. Обидві змінні працюють у кожному рядку.',
  sizeNote:'Розмір тексту або блока в пікселях.', weightNote:'Насиченість шрифту; більше значення — жирніше.', colorNote:'Колір цього елемента.',
  opacityNote:'0 — прозоро, 1 — повністю видимо.', radiusNote:'Заокруглення кутів у пікселях.', winBgNote:'Колір фону вікна алерта.',
  borderNote:'Колір і товщина рамки вікна алерта.', shadowNote:'Сила зовнішньої тіні; 0 прибирає тінь.', paddingNote:'Відступ між рамкою вікна та вмістом.',
  durationNote:'Скільки часу алерт залишається на екрані.', animInNote:'Як алерт з’являється на екрані.', animOutNote:'Як алерт зникає з екрана.', speedNote:'Тривалість анімації в мілісекундах.',
  mediaUrlNote:'Локальна або зовнішня адреса медіа. Кнопка відкриває вибір файлу.', uploadNote:'Завантажити локальний медіафайл.', mediaPosNote:'Розташування медіа відносно вікна алерта.',
  mediaSizeNote:'Ширина й висота медіаблока в пікселях.', mediaZoomNote:'Масштаб медіа у відсотках. Прокрутіть над переглядом, щоб змінити.', centerNote:'Повернути медіа в центр.',
  exportNote:'Завантажити поточний набір алертів у JSON.', importNote:'Завантажити набір алертів із JSON-файлу.', resetNote:'Скинути вибраний алерт до вбудованих значень за замовчуванням.',
  previewNote:'Прокрутіть над переглядом, щоб змінити масштаб; перетягніть, щоб змістити; двічі клацніть, щоб центрувати.'
});
let lang = localStorage.getItem('lm_lang') || 'en';
const t = k => (DICT[lang] && DICT[lang][k]) || (DICT.en[k] || k);
function countLabel(n) {
  if (lang === 'ua') {
    const one = n % 10, hundred = n % 100;
    return n + ' ' + t(one === 1 && hundred !== 11 ? 'cueSingular' : (one >= 2 && one <= 4 && !(hundred >= 12 && hundred <= 14) ? 'cueFew' : 'cuePlural'));
  }
  return n + ' ' + t(n === 1 ? 'cueSingular' : 'cuePlural');
}
function applyLang() { lang = localStorage.getItem('lm_lang') || lang; document.querySelectorAll('[data-i18n]').forEach(el => { el.textContent = t(el.dataset.i18n); });
  if ($('stage')) $('stage').setAttribute('aria-label', t('monitorTitle'));
  if ($('rules')) $('rules').setAttribute('aria-label', t('variation'));
  document.querySelectorAll('#langseg button').forEach(b => b.classList.toggle('on', b.dataset.lang === lang));
  const adminStatus = document.querySelector('.nt-masthead__status span:last-child');
  if (adminStatus) adminStatus.textContent = lang === 'ua' ? '\u041b\u043e\u043a\u0430\u043b\u044c\u043d\u0435 \u043a\u0435\u0440\u0443\u0432\u0430\u043d\u043d\u044f' : 'Local admin';
  document.documentElement.lang = lang === 'ua' ? 'uk' : 'en'; renderRules(); updateRuleOpts(); refreshSelLabels(); if (settingsLoaded) preview(); document.dispatchEvent(new CustomEvent('notify:lang')); }
/* language is applied by applyLang() below (head.js calls window.applyLang) */

let rules = [], sel = 0, settingsLoaded = false;
function newId() { return 'r' + Math.random().toString(36).slice(2, 8); }
function ensureTestRule() { if (!rules.some(r => r.id === 'test')) rules.push({ id: 'test', label: '', min: 0, max: 0, pool: false, style: {}, test_media_url: '' }); }
function ensureStarterBank() {
  if (rules.length !== 1 || rules[0].id !== 'default') return;
  const starter = Array.from({ length: 7 }, () => ({ id: newId(), label: '', min: 100, max: 100, pool: false, style: {} }));
  rules = starter.concat(rules);
  sel = 0;
}
function renderRules() { const box = $('rules'); box.innerHTML = '';
  const count = $('cueCount'); if (count) count.textContent = countLabel(rules.length);
  const active = rules[sel]; const activeName = $('activeCueName');
  const displayNameOf = (r, i) => r.id === 'default' ? t('defaultLabel') : r.id === 'test' ? t('testRuleLabel') : (r.label || t('name') + ' ' + (i + 1));
  if (activeName) activeName.textContent = active ? displayNameOf(active, sel) : '—';
  const range = $('ruleRange');
  if (range) range.textContent = active ? `${active.min || 0} ${active.max > 0 ? '– ' + active.max : '+'}` : '—';
  rules.forEach((r, i) => {
    const isDef = r.id === 'default';
    const displayName = displayNameOf(r, i);
    const row = document.createElement('div');
    row.className = 'nt-cue' + (isDef ? ' nt-cue--default' : '');
    row.tabIndex = 0;
    row.dataset.ruleId = r.id;
    row.setAttribute('role', 'listitem');
    row.setAttribute('aria-current', i === sel ? 'true' : 'false');
    row.setAttribute('aria-label', t('selectVariation') + ': ' + displayName);
    row.onkeydown = e => { if (e.target !== row || (e.key !== 'Enter' && e.key !== ' ')) return; e.preventDefault(); sel = i; loadStyleToUI(); renderRules(); };
    row.onclick = () => { sel = i; loadStyleToUI(); renderRules(); };

    const body = document.createElement('div');
    body.className = 'nt-cue__body';
    const lab = document.createElement('div');
    lab.className = 'nt-rule__label';
    const nm = document.createElement('span');
    nm.className = 'nt-rule__name';
    nm.textContent = displayName;
    const rng = document.createElement('span');
    rng.className = 'nt-rule__range' + (r.pool ? ' is-pool' : '');
    rng.textContent = r.pool ? t('addedToPool') : `${r.min || 0} ₴ / ${(r.max > 0 ? r.max + ' ₴' : '∞')}`;
    lab.append(nm, rng);
    body.append(lab);

    const test = document.createElement('button');
    test.className = 'nt-cue__test';
    test.type = 'button';
    test.textContent = t('testShort');
    test.title = t('testVariation');
    test.setAttribute('aria-label', t('testVariation'));
    test.onclick = (e) => { e.stopPropagation(); testRule(r); };

    const isTest = r.id === 'test';
    const second = document.createElement('button');
    second.className = 'nt-cue__test nt-cue__test--yt';
    second.type = 'button';
    second.textContent = isTest ? t('testMD') : t('testYT');
    second.title = isTest ? t('testMDTitle') : t('testYTTitle');
    second.setAttribute('aria-label', second.title);
    second.onclick = (e) => { e.stopPropagation(); testRule(r, isTest ? 'md' : 'yt'); };

    row.append(body, test, second);
    if (isDef || isTest) {
      /* Decorative side stack: it reads as part of the row, but has no actions. */
      const side = document.createElement('div');
      side.className = 'nt-cue__side nt-cue__side--default';
      side.setAttribute('aria-hidden', 'true');
      const top = document.createElement('span');
      top.className = 'nt-cue__ghost nt-cue__ghost--top';
      const bottom = document.createElement('span');
      bottom.className = 'nt-cue__ghost nt-cue__ghost--bottom';
      side.append(top, bottom);
      row.append(side);
    } else {
      const side = document.createElement('div');
      side.className = 'nt-cue__side';
      const pool = document.createElement('button');
      pool.className = 'nt-cue__toggle nt-cue__toggle--pool';
      pool.type = 'button';
      pool.textContent = r.pool ? '✓' : '+';
      pool.title = r.pool ? t('poolRemove') : t('poolLabel');
      pool.setAttribute('aria-label', pool.title);
      pool.setAttribute('aria-pressed', r.pool ? 'true' : 'false');
      pool.onclick = (e) => { e.stopPropagation(); r.pool = !r.pool; renderRules(); if (sel === i) loadStyleToUI(); };
      const del = document.createElement('button');
      del.className = 'nt-cue__toggle nt-cue__toggle--delete';
      del.type = 'button';
      del.textContent = '×';
      del.title = t('deleteVariation');
      del.setAttribute('aria-label', t('deleteVariation'));
      del.onclick = (e) => { e.stopPropagation(); rules.splice(i, 1); if (sel >= rules.length) sel = 0; loadStyleToUI(); renderRules(); };
      side.append(del, pool);
      row.append(side);
    }
    box.append(row);
  }); }
function testRule(r, mode) { const body = { amount: sampleAmount(r), name: t('previewDonor'), ruleId: r.id, style: r.style || {} }; if (mode === 'yt') body.youtube = 'https://www.youtube.com/watch?v=teUWsONJkk8'; if (mode === 'md') body.mediaUrl = ($('test_media_url')?.value || '').trim(); fetch('/api/test', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }); }
$('addrule').onclick = () => { const idx = Math.max(0, rules.findIndex(r => r.id === 'default')); rules.splice(idx, 0, { id: newId(), label: '', min: 100, max: 100, pool: false, style: {} }); sel = idx; loadStyleToUI(); renderRules(); };
const clampRange = v => Math.min(29999, Math.max(10, Math.round(Number(v)) || 10));
['rule_min','rule_max'].forEach(id => $(id).addEventListener('input', () => { if (!rules[sel]) return; rules[sel].min = clampRange($('rule_min').value); rules[sel].max = clampRange($('rule_max').value); renderRules(); }));
$('rule_label').addEventListener('input', e => { if (!rules[sel] || rules[sel].id === 'default') return; rules[sel].label = e.target.value.trim(); renderRules(); });
/* YouTube settings live on the Donate Page Customization page */
/* add-to-pool is toggled in each rule row */
function readStyle() { const s = {}; for (const k of STYLE_KEYS) { const el = $(k); if (!el) continue; if (BOOL.has(k)) s[k] = el.checked; else if (NUM.has(k)) s[k] = Number(el.value); else s[k] = el.value; }
  const cur = (rules[sel] && rules[sel].style) || {};
  s.media_pan_x = cur.media_pan_x || 0; s.media_pan_y = cur.media_pan_y || 0;
  s.yt_min = cur.yt_min || 0; s.yt_views = cur.yt_views || 0; s.yt_max_min = cur.yt_max_min || 0; s.yt_playlists = !!cur.yt_playlists;
  for (const k of ['media_youtube','media_instagram','media_tiktok','media_twitch','media_reddit','media_x']) s[k] = cur[k] !== false;
  return s; }
function writeStyle(s) { const m = Object.assign({}, DEFAULTS, s || {}); for (const k of STYLE_KEYS) { const el = $(k); if (!el) continue;
  if (BOOL.has(k)) el.checked = !!m[k];
  else if (k === 'media_w' || k === 'media_h') el.value = (Number(m[k]) > 0 ? m[k] : 320);
  else el.value = (m[k] ?? ''); }
  document.dispatchEvent(new CustomEvent('notify:style')); }
function updateRuleOpts() { const r = rules[sel]; if (!r) return; const isDef = r.id === 'default';
  if (isDef) { r.min = 10; r.max = 29999; $('rule_min').value = 10; $('rule_max').value = 29999; }
  const lock = isDef || r.pool;
  $('rule_min').disabled = lock; $('rule_max').disabled = lock;
  const nameInput = $('rule_label');
  if (nameInput) { nameInput.disabled = isDef; nameInput.placeholder = isDef ? t('defaultLabel') : t('name'); }
  $('rulenote').textContent = isDef || !r.pool ? t('fixedCueNote') : t('poolCueNote');
}
function syncTestTab() { const isTest = rules[sel] && rules[sel].id === 'test'; const tab = $('tab-test'), panel = $('panel-test'); if (tab) tab.hidden = !isTest; if (panel) panel.hidden = !isTest || !tab.classList.contains('is-active'); if (!isTest && tab && tab.classList.contains('is-active')) { const copy = $('tab-copy'); if (copy) copy.click(); } }
function loadStyleToUI() { if (rules[sel]) { writeStyle(rules[sel].style || {}); $('test_media_url').value = rules[sel].test_media_url || '';
    $('rule_min').value = rules[sel].min || 0; $('rule_max').value = rules[sel].max || 0;
    $('rule_label').value = rules[sel].id === 'default' ? '' : (rules[sel].label || ''); } updateRuleOpts(); updateMediaLocks(); syncTestTab(); preview(); }

function preview() {
  if (!rules[sel]) return;
  const s = readStyle(); rules[sel].style = s;
  const pv = Object.assign({}, s);
  if (lang === 'en' && pv.money_text === '%donate% задонатив %money%') pv.money_text = '%donate% donated %money%';
  const data = { name: t('previewDonor'), amount: 100, currency: '₴', message: lang==='ua' ? 'Приклад повідомлення в ефірі, який може бути довгим і переноситься на кілька рядків без обрізання' : 'Example on-stream message that can be long and wraps across several lines without being cut off' };
  const stage = $('stage'); stage.innerHTML = '';
  const r = Alert.build(data, pv, { preview: true });
  stage.append(r.notif);
  if (r.red) requestAnimationFrame(() => Alert.fitMessage(r.red, r.msgH, r.msgBase));
}
function setDis(id, dis) { const inp = $(id); if (inp) inp.disabled = dis; const b = document.querySelector('[data-up="' + id.replace('_url', '') + '"]'); if (b) b.disabled = dis; }
function updateMediaLocks() {
  const img = ($('image_url').value || '').trim(), vid = ($('video_url').value || '').trim(), snd = ($('sound_url').value || '').trim();
  const lockVideo = !!img || !!snd, lockImgSnd = !!vid;
  setDis('video_url', lockVideo); setDis('image_url', lockImgSnd); setDis('sound_url', lockImgSnd);
}
['image_url', 'video_url', 'sound_url'].forEach(id => $(id).addEventListener('input', updateMediaLocks));
let upKind = 'image';
document.querySelectorAll('[data-up]').forEach(b => b.onclick = () => { upKind = b.dataset.up; $('file').click(); });
$('file').onchange = async e => { const f = e.target.files && e.target.files[0]; if (!f) return; const safe = f.name.replace(/[^A-Za-z0-9._-]/g, '_');
  try { const r = await fetch('/api/upload?name=' + encodeURIComponent(safe), { method: 'POST', body: f }); const d = await r.json();
    if (d.ok) { $(upKind + '_url').value = d.url; preview(); $('status').textContent = '↑ ' + d.url; } else $('status').textContent = 'upload error'; } catch { $('status').textContent = 'upload error'; } };
/* lang seg wired in extraTools */
let TTS = null;
function fillTtsVoices() {
  const mode = $('tts_mode').value; const all = (TTS && TTS.voices) || [];
  const azuk = all.filter(v => v.id === 'azure:uk-UA-PolinaNeural' || v.id === 'azure:uk-UA-OstapNeural').map(v => v.id);
  const azus = all.filter(v => v.id === 'azure:en-US-JennyNeural' || v.id === 'azure:en-US-GuyNeural').map(v => v.id);
  const hasKey = (TTS && TTS.tts && TTS.tts.hasAzure) || $('azure_key').value.trim() !== '';
  const allow = mode === 'off' ? [] : mode === 'google:uk' ? ['google:uk'] : mode === 'google:us' ? ['google:us']
    : mode === 'azure:uk' ? azuk : mode === 'azure:us' ? azus : [];
  const sel = $('tts_voice'); const cell = sel.closest('.cell'); const prev = sel.value; sel.innerHTML = '';
  const multi = (mode === 'azure:uk' || mode === 'azure:us');
  if (multi) sel.append(new Option(t('allVoices'), ''));
  all.filter(v => allow.includes(v.id)).forEach(v => sel.append(new Option(v.label, v.id)));
  if (prev && (prev === '' || allow.includes(prev))) sel.value = prev;
  if (cell) cell.style.display = multi ? '' : 'none';
  const isAz = (mode === 'azure:uk' || mode === 'azure:us');
  $('azure_cell').style.display = isAz ? '' : 'none';
  $('azure_reg_cell').style.display = isAz ? '' : 'none';
  $('azure_help').style.display = isAz ? '' : 'none';
  const warn = document.getElementById('azure_warn');
  if (warn) { const need = isAz && !hasKey; warn.style.display = need ? '' : 'none'; }
  if (typeof refreshSelLabels === 'function') refreshSelLabels();
}
async function loadTts() {
  try { TTS = await (await fetch('/api/tts')).json(); const tc = TTS.tts || {};
    const known = ['google:uk', 'google:us', 'azure:uk', 'azure:us', 'off'];
    const legacy = (tc.voice === 'google:us') ? 'google:us' : 'google:uk';
    $('tts_mode').value = known.includes(tc.mode) ? tc.mode : legacy;
    $('tts_read').value = tc.readMode || 'announcement';
    $('tts_speed').value = tc.speed || 1.2;
    if (tc.azureRegion) $('azure_region').value = tc.azureRegion;
    if (tc.hasAzure) $('azure_key').placeholder = '•••••• saved';
    fillTtsVoices(); if (tc.voice) $('tts_voice').value = tc.voice; refreshSelLabels(); } catch {}
}
$('tts_mode').addEventListener('change', fillTtsVoices);
window.savePage = async () => { if (rules[sel]) { rules[sel].style = readStyle(); if (rules[sel].id === 'test') rules[sel].test_media_url = ($('test_media_url')?.value || '').trim(); }
  try { const r = await fetch('/api/notify', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ rules }) }); const d = await r.json();
    await fetch('/api/tts', { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ mode: $('tts_mode').value, voice: $('tts_voice').value, readMode: $('tts_read').value,
        speed: Number($('tts_speed').value) || 1.2,
        azureKey: $('azure_key').value, azureRegion: $('azure_region').value }) });
    $('status').className = 'status' + (d.ok ? ' ok' : ' err'); $('status').textContent = d.ok ? t('saved') : ('error: ' + (d.error || ''));
    if (d.ok) { const nr = await (await fetch('/api/notify')).json(); if (nr.rules) { rules = nr.rules; loadStyleToUI(); renderRules(); } } } catch { $('status').className='status err'; $('status').textContent = 'save error'; } };
/* test wired in extraTools */
function sampleAmount(r) { if (r.id === 'default') return 100; if (r.max > r.min && r.min > 0) return r.min; if (r.max > 0) return r.max; return r.min > 0 ? r.min : 100; }
$('export').onclick = () => { const blob = new Blob([JSON.stringify({ rules }, null, 2)], { type: 'application/json' }); const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = 'localmonodonate-alerts.json'; a.click(); };
$('import').onclick = () => { const inp = document.createElement('input'); inp.type='file'; inp.accept='application/json'; inp.onchange = () => { const f=inp.files[0]; if(!f) return; const rd=new FileReader(); rd.onload=()=>{ try{ const j=JSON.parse(rd.result); if (j.rules) { rules=j.rules; sel=0; loadStyleToUI(); renderRules(); } }catch{} }; rd.readAsText(f); }; inp.click(); };
$('reset').onclick = () => { if (!rules[sel]) return; if (!confirm(t('confirmReset'))) return;
  rules[sel].style = JSON.parse(JSON.stringify(DEFAULTS)); loadStyleToUI(); renderRules();
  $('status').className = 'status ok'; $('status').textContent = t('resetDone'); };
let pvT = null;
document.querySelectorAll('input,select').forEach(el => { if (el.id && el.id !== 'file' && el.id !== 'bgfile') el.addEventListener('input', () => { clearTimeout(pvT); pvT = setTimeout(preview, 90); }); });
function refreshSelLabels() {}
(async () => {
  try { const fonts = await (await fetch('/api/fonts')).json(); const list = (fonts.fonts && fonts.fonts.length ? fonts.fonts : ['Segoe UI','Arial']);
    ['blue_font','green_font','msg_font'].forEach(id => { const s2 = $(id); s2.innerHTML = ''; for (const f of list) { const o = document.createElement('option'); o.value = f; o.textContent = f; s2.append(o); } }); } catch {}
  try { const n = await (await fetch('/api/notify')).json(); rules = (n.rules && n.rules.length) ? n.rules : [ { id:'default', label:'', min:0, max:0, style:{} } ]; sel = 0; } catch { rules = [ { id:'default', label:'', min:0, max:0, style:{} } ]; }
  ensureStarterBank(); ensureTestRule(); applyLang(); loadStyleToUI(); settingsLoaded = true; loadTts(); })();
window.applyLang = applyLang;
window.refreshPreview = function () { try { preview(); } catch {} };
/* Presentation hook for media drag-to-pan. */
window.getMediaPan = function () {
  const st = (rules[sel] && rules[sel].style) || {};
  return { x: Number(st.media_pan_x) || 0, y: Number(st.media_pan_y) || 0 };
};
window.setMediaPan = function (x, y) {
  if (!rules[sel]) return;
  const st = rules[sel].style || {};
  st.media_pan_x = Math.round(Number(x) || 0);
  st.media_pan_y = Math.round(Number(y) || 0);
  rules[sel].style = st;
  preview();
};


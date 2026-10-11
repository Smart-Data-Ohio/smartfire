/**
 * The sign-in fields' translation lists, as the retained page prints them beside each label
 * (crates/view_kit/src/helpers/translations_table.rs, `user_name`, `email_address` and `password`).
 */
export interface Translation {
  readonly flag: string;
  readonly text: string;
}

export const NAME_TRANSLATIONS: readonly Translation[] = [
  { flag: "🇺🇸", text: "Enter your name" },
  { flag: "🇪🇸", text: "Introduce tu nombre" },
  { flag: "🇫🇷", text: "Entrez votre nom" },
  { flag: "🇮🇳", text: "अपना नाम दर्ज करें" },
  { flag: "🇩🇪", text: "Geben Sie Ihren Namen ein" },
  { flag: "🇧🇷", text: "Insira seu nome" },
  { flag: "🇯🇵", text: "お名前を入力してください" },
];

export const EMAIL_TRANSLATIONS: readonly Translation[] = [
  { flag: "🇺🇸", text: "Enter your email address" },
  { flag: "🇪🇸", text: "Introduce tu correo electrónico" },
  { flag: "🇫🇷", text: "Entrez votre adresse courriel" },
  { flag: "🇮🇳", text: "अपना ईमेल पता दर्ज करें" },
  { flag: "🇩🇪", text: "Geben Sie Ihre E-Mail-Adresse ein" },
  { flag: "🇧🇷", text: "Insira seu endereço de email" },
  { flag: "🇯🇵", text: "メールアドレスを入力してください" },
];

export const PASSWORD_TRANSLATIONS: readonly Translation[] = [
  { flag: "🇺🇸", text: "Enter your password" },
  { flag: "🇪🇸", text: "Introduce tu contraseña" },
  { flag: "🇫🇷", text: "Saisissez votre mot de passe" },
  { flag: "🇮🇳", text: "अपना पासवर्ड दर्ज करें" },
  { flag: "🇩🇪", text: "Geben Sie Ihr Passwort ein" },
  { flag: "🇧🇷", text: "Insira sua senha" },
  { flag: "🇯🇵", text: "パスワードを入力してください" },
];

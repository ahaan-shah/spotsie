---
title: Your Spotify App
description: "Make the free Spotify app Spotsie signs in through, and fix what can go wrong."
nav_order: 6
---

## Why Spotsie needs your own app

Spotify only lets an app like Spotsie read your library through an app
registered on its developer website. Spotsie used to borrow one that other
music players shared, but Spotify no longer accepts new sign-ins through it.
So each listener makes their own. It is free, takes about two minutes, needs
no code, and Spotify requires the account that makes it to have Premium.

Your own app also has its own request allowance, so you are not slowed down
when other people's players are busy.

Onboarding walks you through this the first time you open Spotsie. The steps
are the same here.

## Make a Spotify app

1. Open the [Spotify developer dashboard](https://developer.spotify.com/dashboard)
   and log in with your Spotify account. Accept the developer terms if
   Spotify asks.
2. Click **Create app**. Any name and description will do; nobody else
   sees them.
3. Under **Redirect URIs**, add exactly:

   ```
   http://127.0.0.1:8989/login
   ```

4. Tick **Web API**, accept the terms, and click **Save**.
5. The app's page shows its **Client ID** under **Basic Information**. Copy it.

## Sign in with it

Paste the Client ID into Spotsie and press **Sign in with Spotify**. The field
shows a check once what you pasted looks like a Client ID (32 letters and
digits). Before your browser opens, Spotsie asks Spotify whether it knows the
Client ID, and tells you if it does not. Approve Spotsie on Spotify's page and
Spotsie carries on by itself.

Spotsie remembers the app. When you sign out and back in, onboarding starts at
**Welcome back** with your Client ID already filled in; **Set up a different
app** goes back to the steps.

To change the app while signed in, open **Settings**, find **Your Spotify
app**, paste the new Client ID, and click **Authorize**. Removing the app is
the same as signing out, so Settings no longer offers it on its own.

## Signed in before 0.1.4

If you signed in through the old shared app, you stay signed in until Spotify
refuses it. Spotsie offers to move you to your own app once, and Settings shows
the same. When Spotify does refuse the old sign-in, onboarding says so and
opens at the steps above.

## If something goes wrong

- **Spotify says "Invalid redirect URI".** The redirect URI in your app does
  not match. Add `http://127.0.0.1:8989/login` exactly, with no trailing slash,
  save, and sign in again. Spotify shows this on its own page and never tells
  Spotsie, so the browser step reminds you of it.
- **"Spotify doesn't know this Client ID".** Copy the Client ID again from your
  app's page. It is not the Client Secret.
- **"Spotify didn't let this account use the app".** Sign in with the Spotify
  account that made the app, and check that it has Premium. In Development
  Mode an app only lets in its owner and the users they add.
- **Port 8989 is in use.** Another program is using Spotsie's sign-in port.
  Close it and try again.

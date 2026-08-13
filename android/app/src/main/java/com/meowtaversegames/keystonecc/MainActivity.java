package com.meowtaversegames.keystonecc;

import com.google.androidgamesdk.GameActivity;

/** GameActivity forwards Android lifecycle and physical keyboard input to Bevy. */
public final class MainActivity extends GameActivity {
    static {
        System.loadLibrary("keystone_cc");
    }
}

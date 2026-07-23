[0000]   BLOCK (exit -> @02D2)
[0004]     AWAIT gameflag_252A
[0005]     GUARD active_actor == Scruter_Jo.talk (related 40)
[000A]     GUARD rec_0F4E == 3488
[000F]     ENDIF
[0010]     SAY "ME SCRUTER JO: SCANNING STRANGER...XRAY....STOP"  '[voice 1]
[0022]     SAY "ZONE ULTRAFORBIDDEN YOU NOT STAY HERE ... YOU GO AWAY ..."  '[voice 1]
[0040]     SAY "FIRST WARNING ..."  '[voice 1, skip 1]
[004E]     SETCHAR slot 4 = "scrut"
[0057]     IF-BLOCK (exit -> @00AC)
[005A]       GUARD rec_0740 > 1
[0061]       ENDIF
[0062]       SAY "Oh no ! Him again ... Commander , I don't feel happy about that SCRUT robot ..."
[008C]       SAY "We'll need to be smart , cunning , intelligent and clever ..."
    END
[00AC]     IF-BLOCK (exit -> @00D7)
[00AF]       GUARD rec_0740 == 2
[00B6]       ENDIF
[00B7]       SAY "Watch out , Commander ... Second try ... Don't blow it ..."
    END
[00D7]     IF-BLOCK (exit -> @0104)
[00DA]       GUARD rec_0740 == 3
[00E1]       ENDIF
[00E2]       SAY "I think I found the code , Commander... It's the number 9 ..."
    END
[0104]     SAY "You give identity code : word_65535 robyx code ulikan 69 exxos electret 666 9"  '[voice 1]
[012A]     IF-BLOCK (exit -> @01BE)
[012D]       GUARD concept == "exxos"
[0130]       ENDIF
[0131]       SAY "Yes ! Code be EXXOS. You be very Important Being ..."  '[voice 2]
[014F]       SAY "Nice work , Commmander , I've got to hand it to you ... How did you figure it out ?"
[017F]       SAY "Ol' Turkey Face Bob'll be proud of you when I tell him ..."
[01A1]       SAY "You did give code . You are the MASTER ..."  '[voice 1]
[01BD]       CLEAR concept_alt
    END
[01BE]     IF-BLOCK (exit -> @0227)
[01C1]       GUARD NOT concept == "exxos"
[01C5]       ENDIF
[01C6]       SAY "You unauthorized . Me exterminate you now ..."  '[voice 3]
[01DE]       SAY "Uh oh Commander . He's gonna zap the Orxx..."
[01F8]       SAY "Bye bye , unauthorized strangers ..."  '[voice 4]
[020C]       SAY "BAAANG!!!"  '[skip 3]
[0216]       LOADSTR "explo3.hnm"
[0223]       END PRESENTATION Scruter_Jo.talk
[0226]       CLEAR concept_alt
    END
[0227]     SAY "GIVE ME YOUR ORDERS , MASTER . I am yours ..."  '[voice 5]
[0245]     SAY "Commander , We found the code for programming him ..."
[0261]     SAY "Let's teleport him , Commander . I ought to take a good look at him ... word_65535 teleport refuse"
[0291]     IF-BLOCK (exit -> @02B3)
[0294]       GUARD concept == "teleport"
[0297]       ENDIF
[0298]       SAY "TELEPORT SCRUTER JO TO CRYOBOX"  '[skip 3]
[02AA]       rec_0722 = 65535
[02AF]       END PRESENTATION Scruter_Jo.talk
[02B2]       CLEAR concept_alt
    END
[02B3]     IF-BLOCK (exit -> @02D2)
[02B6]       GUARD concept == "refuse"
[02B9]       ENDIF
[02BA]       SAY "You're the commander , Commander ..."  '[skip 2]
[02CE]       CLEAR concept_alt
[02CF]       END PRESENTATION Scruter_Jo.talk
    END
  END
[02D2]   BLOCK (exit -> @0381)
[02D6]     AWAIT gameflag_274F
[02D7]     GUARD active_actor == Scruter_Jo.talk (related 40)
[02DC]     ENDIF
[02DD]     SAY "I've reprogrammed him , Commander ... And I've discovered something weird ..."
[02FD]     SAY "These SCRUT robots use a psychic structure based on CYBERSPACE ..."
[031B]     SAY "I've rigged a system so you can enter it , Commander... That way you can get some BIONIUM..."
[0347]     SAY "The energy source computers are wild about..."
[035D]     SAY "Ask SCRUTER JO a few questions , Commander . You'll understand ..."  '[skip 1]
[037D]     POKE [0x02D3] = 0
  END
[0381]   BLOCK (exit -> @06B9)
[0385]     AWAIT gameflag_274F
[0386]     GUARD active_actor == Scruter_Jo.talk (related 40)
[038B]     ENDIF
[038C]     SAY "Commander you go get BIONIUM in CYBERSPACE of SCRUTER JO..."  '[voice 19]
[03A8]     IF-BLOCK (exit -> @0549)
[03AB]       GUARD compris == 0
[03B2]       ENDIF
[03B3]       SAY "Me explain to you how get BIONIUM..."
[03C9]       SAY "You find BIOXX . Bioxx be small energy creatures ..."
[03E5]       SAY "You touch BIOXX with hand ."  '[voice 1]
[03F9]       SAY "Sounds like a piece of cake to me , Commander !!"
[0417]       SAY "If you touch BIOXX, you CAPTURE BIOXX on tip of your finger ..."  '[voice 2]
[0439]       SAY "Catch him on the tip of your finger !!! Sounds easy as pie , Commander ..."
[0461]       SAY "Then you can carry BIOXX to cybernetic MANTAS"  '[voice 3]
[0479]       SAY "You place BIOXX in belly of Manta..."  '[voice 5]
[048F]       SAY "BIOXX stay stuck to MANTAS..."  '[voice 4]
[04A1]       SAY "I'd love to see that ..."
[04B5]       SAY "Mantas change BIOXX into BIONIUM..."  '[voice 6]
[04C7]       SAY "More BIOXX you give to Mantas, more BIONIUM you get ..."  '[voice 5]
[04E5]       SAY "Yes !!! BIONIUM ... I can taste it already ..."
[0501]       SAY "To come back from CYBERSPACE , you touch BLUE BOX ...."  '[voice 4]
[051F]       SAY "You understand ?"  '[voice 6]
[052D]       SAY "We understand perfectly , Mister SCRUTER JO... Right , Commander?"
    END
[0549]     SAY "YOU go , Commander ..."  '[voice 20]
[055B]     SAY "Ahh! Me feel better ..."  '[voice 21]
[056D]     IF-BLOCK (exit -> @0613)
[0570]       GUARD vbio > 0
[0577]       ENDIF
[0578]       SAY "Good work ... You did succeed ..."  '[voice 2]
[058E]       SAY "You did get BIONIUM..."  '[voice 3]
[059E]       SAY "YES !!! Commander , remind me to tell you you're a champ ..."
[05C0]       SAY "This BIONIUM is extraordinary . My clock frequency's through the roof ..."
[05E0]       SAY "I feel even smarter ... I can feel I'll be a great help to you , Commander ..."  '[skip 1]
[060C]       compris = 1
    END
[0613]     IF-BLOCK (exit -> @069C)
[0616]       GUARD vbio == 0
[061D]       ENDIF
[061E]       SAY "Not good , friend ... You fail ..."  '[voice 4]
[0636]       SAY "Commander, you didn't understand the technique ..."
[064C]       SAY "I need BIONIUM, Commander . It makes me smarter ..."
[0668]       SAY "Ha! Ha! You need much much BIONIUM . Ha! Ha!..."  '[voice 4]
[0684]       SAY "Why don't you shut up , wiseguy !!!"
    END
[069C]     SAY "Bye bye , Commander . Me return to CYBERSPACE..."  '[voice 7, skip 1]
[06B6]     END PRESENTATION Scruter_Jo.talk
  END
[06B9]   BLOCK (exit -> @076C)
[06BD]     AWAIT presentation
[06BE]     GUARD active_actor == menu.talk (related 40)
[06C3]     GUARD rec_0332 == 65535
[06C8]     ENDIF
[06C9]     SAY ""IMPROVED MENU""
[06D5]     SAY "Today CHEF BRONKO has laid on for you :"
[06EF]     SAY "Tasty MURFFALO soup Bronko-style ."
[0701]     SAY "MURFFALO kidneys Bronko-style ."
[0711]     SAY "MURFFALO hamburger with Bar-B-Q recycled-oil dip ."
[0727]     SAY "Smooth MURFFALO-chip ice cream ."
[0739]     SAY "Recycled water"
[0745]     SAY "Chef Bronko says ... Burping's bad manners ! ..."
[075F]     SAY "stop"  '[skip 1]
[0769]     END PRESENTATION menu.talk
  END
[076C]   BLOCK (exit -> @081C)
[0770]     AWAIT presentation
[0771]     GUARD active_actor == menu.talk (related 40)
[0776]     GUARD NOT rec_0332 == 65535
[077C]     ENDIF
[077D]     SAY ""MENU""
[0787]     SAY "Today's fare :"
[0795]     SAY "PLASMA soup HONK-style ."
[07A5]     SAY "WRIGGLER belly in slobber sauce ."
[07B9]     SAY "Jellied URTIKAN with MURFFALO bone marrow ."
[07CF]     SAY "GLOK eye pie ."
[07DF]     SAY "Recycled water"
[07EB]     SAY "The chef says ... Don't eat with your mouth full ! ..."
[080B]     SAY "Stop"  '[skip 2]
[0815]     POKE [0x076D] = 0
[0819]     END PRESENTATION menu.talk
  END
[081C]   BLOCK (exit -> @08C6)
[0820]     AWAIT presentation
[0821]     GUARD active_actor == menu.talk (related 40)
[0826]     GUARD NOT rec_0332 == 65535
[082C]     ENDIF
[082D]     SAY ""MENU""
[0837]     SAY "Today's fare :"
[0845]     SAY "PLASMA soup HONK-style ."
[0855]     SAY "WRIGGLER snout stew ."
[0865]     SAY "URTIKAN seeds in MURFFALO venom ."
[0879]     SAY "GLOK juice dessert ."
[0889]     SAY "Recycled water"
[0895]     SAY "The chef says ... Don't talk with your mouth open ! ..."
[08B5]     SAY "stop"  '[skip 2]
[08BF]     POKE [0x081D] = 0
[08C3]     END PRESENTATION menu.talk
  END
[08C6]   BLOCK (exit -> @0972)
[08CA]     AWAIT presentation
[08CB]     GUARD NOT rec_0332 == 65535
[08D1]     GUARD active_actor == menu.talk (related 40)
[08D6]     ENDIF
[08D7]     SAY ""MENU""
[08E1]     SAY "Today's fare :"
[08EF]     SAY "PLASMA soup HONK-style ."
[08FF]     SAY "WRIGGLER feet in emulsive sauce ."
[0913]     SAY "URTIKAN leaves in MURFFALO sweat ."
[0927]     SAY "GLOK flake dessert ."
[0937]     SAY "Recycled water"
[0943]     SAY "The chef says ... Somebody didn't finish his wrigglers yesterday ..."
[0961]     SAY "stop"  '[skip 2]
[096B]     POKE [0x08C7] = 0
[096F]     END PRESENTATION menu.talk
  END
[0972]   BLOCK (exit -> @0A26)
[0976]     AWAIT presentation
[0977]     GUARD active_actor == menu.talk (related 40)
[097C]     GUARD NOT rec_0332 == 65535
[0982]     ENDIF
[0983]     SAY ""MENU""
[098D]     SAY "Today's fare :"
[099B]     SAY "HONK-style PLASMA soup ."
[09AB]     SAY "WRIGGLER brain , stewed in its own juice ."
[09C5]     SAY "URTIKAN trunk , stuffed with MURFFALO liver ."
[09DD]     SAY "GLOK dee-lite ."
[09EB]     SAY "Recycled water"
[09F7]     SAY "The chef says ... Plenty more in the kitchen ! ..."
[0A15]     SAY "stop"  '[skip 2]
[0A1F]     POKE [0x0973] = 0
[0A23]     END PRESENTATION menu.talk
  END
[0A26]   BLOCK (exit -> @0AE2)
[0A2A]     AWAIT presentation
[0A2B]     GUARD active_actor == menu.talk (related 40)
[0A30]     GUARD NOT rec_0332 == 65535
[0A36]     ENDIF
[0A37]     SAY ""IMPROVED MENU""
[0A43]     SAY "Today's fare :"
[0A51]     SAY "Soup of PLASMA HONK-style ."
[0A63]     SAY "WRIGGLER hearts in green blood coagulate ."
[0A79]     SAY "URTIKAN roots , deep fried in recycled oil ."
[0A93]     SAY "Candied GLOK tongue ."
[0AA3]     SAY "Recycled water"
[0AAF]     SAY "The chef says ... You eat what you are ! ..."
[0ACD]     SAY "stop"  '[skip 3]
[0AD7]     POKE [0x0A27] = 0
[0ADB]     POKE [0x0AE3] = 1
[0ADF]     END PRESENTATION menu.talk
  END
[0AE2]   GOTO @0AFF
[0AE6]   ENDIF
[0AE7]   POKE [0x076D] = 1
[0AEB]   POKE [0x081D] = 1
[0AEF]   POKE [0x08C7] = 1
[0AF3]   POKE [0x0973] = 1
[0AF7]   POKE [0x0A27] = 1
[0AFB]   POKE [0x0AE3] = 0
[0AFF]   BLOCK (exit -> @0B82)
[0B03]     AWAIT presentation
[0B04]     GUARD active_actor == Honk.talk (related 40)
[0B09]     ENDIF
[0B0A]     IF-BLOCK (exit -> @0B37)
[0B0D]       GUARD plus == 0
[0B14]       ENDIF
[0B15]       SAY "Commander , Cap'n Bob's a secretive buzzard ... He is hiding something ..."
    END
[0B37]     IF-BLOCK (exit -> @0B82)
[0B3A]       GUARD plus == 1
[0B41]       ENDIF
[0B42]       SAY "Commander, I hope you're gonna tell me what ol' Turkey face said when he unplugged me ..."
[0B6C]       SAY "It's a secret ... Right Commander ..."
    END
  END
[0B82]   BLOCK (exit -> @0DDF)
[0B86]     AWAIT presentation
[0B87]     GUARD active_actor == Honk.talk (related 40)
[0B8C]     ENDIF
[0B8D]     SAY ""  '[skip 2]
[0B95]     adieu = 0
[0B9C]     state[15] = 100
[0BA0]     IF-BLOCK (exit -> @0BC7)
[0BA3]       GUARD rec_0080 == 0
[0BAA]       ENDIF
[0BAB]       SAY "Commander , remember ol' Bob snoring in the Cryobox ..."
    END
[0BC7]     IF-BLOCK (exit -> @0DDF)
[0BCA]       GUARD rec_0722 == 65535
[0BCF]       ENDIF
[0BD0]       IF-BLOCK (exit -> @0C3D)
[0BD3]         GUARD vbio == 0
[0BDA]         ENDIF
[0BDB]         SAY "Commander , we have no BIONIUM... COMMANDER ... Please"
[0BF5]         SAY "I need that energy ..."
[0C07]         SAY "You have to go into Scruter Jo's CYBERSPACE ..."
[0C21]         SAY "Wake up Scruter_Jo, Commander . He's sleeping in the Cryobox..."
      END
[0C3D]       IF-BLOCK (exit -> @0CA4)
[0C40]         GUARD vbio == 1
[0C47]         ENDIF
[0C48]         SAY "We've only got one dose of BIONIUM left , Commander"
[0C64]         SAY "You should think about a trip to SCRUTER JO's CYBERSPACE..."
[0C80]         SAY "I'm not feeling too sure of myself ... Commander... I really need that energy..."
      END
[0CA4]       IF-BLOCK (exit -> @0CE9)
[0CA7]         GUARD vbio == 2
[0CAE]         ENDIF
[0CAF]         SAY "We've got two doses of BIONIUM left , Commander"
[0CC9]         SAY "It's about time you paid a visit to SCRUTER JO's CYBERSPACE ..."
      END
[0CE9]       IF-BLOCK (exit -> @0D0E)
[0CEC]         GUARD vbio == 3
[0CF3]         ENDIF
[0CF4]         SAY "We've got three doses of BIONIUM left , Commander"
      END
[0D0E]       IF-BLOCK (exit -> @0D33)
[0D11]         GUARD vbio == 4
[0D18]         ENDIF
[0D19]         SAY "We've got four doses of BIONIUM left , Commander"
      END
[0D33]       IF-BLOCK (exit -> @0D58)
[0D36]         GUARD vbio == 5
[0D3D]         ENDIF
[0D3E]         SAY "We've got five doses of BIONIUM left , Commander"
      END
[0D58]       IF-BLOCK (exit -> @0D7D)
[0D5B]         GUARD vbio == 6
[0D62]         ENDIF
[0D63]         SAY "We've got six doses of BIONIUM left , Commander"
      END
[0D7D]       IF-BLOCK (exit -> @0DA2)
[0D80]         GUARD vbio == 7
[0D87]         ENDIF
[0D88]         SAY "We've got seven doses of BIONIUM left , Commander"
      END
[0DA2]       IF-BLOCK (exit -> @0DDF)
[0DA5]         GUARD vbio == 8
[0DAC]         ENDIF
[0DAD]         SAY "We've got eight doses of BIONIUM left , Commander"
[0DC7]         SAY "You're the best of the best , Commander..."
      END
    END
  END
[0DDF]   BLOCK (exit -> @0E49)
[0DE3]     AWAIT presentation
[0DE4]     GUARD active_actor == Honk.talk (related 40)
[0DE9]     GUARD vbio > 2
[0DF0]     GUARD (rec_0B2C & 0x2) == 0
[0DF6]     GUARD rec_0590 > 0
[0DFD]     ENDIF
[0DFE]     SAY "Commander , I'm sure Izwalito knows other planetary coordinates ..."  '[skip 1]
[0E1A]     vbio -= 3
[0E21]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0E45]     POKE [0x0DE0] = 0
  END
[0E49]   BLOCK (exit -> @0EB3)
[0E4D]     AWAIT presentation
[0E4E]     GUARD active_actor == Honk.talk (related 40)
[0E53]     GUARD vbio > 2
[0E5A]     GUARD rec_0230 > 1
[0E61]     GUARD (rec_0EB0 & 0x2) == 0
[0E67]     ENDIF
[0E68]     SAY "Commander, Yoko likes Slimers ... Ask him for information ..."  '[skip 1]
[0E84]     vbio -= 3
[0E8B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0EAF]     POKE [0x0E4A] = 0
  END
[0EB3]   BLOCK (exit -> @0F26)
[0EB7]     AWAIT presentation
[0EB8]     GUARD active_actor == Honk.talk (related 40)
[0EBD]     GUARD vbio > 2
[0EC4]     GUARD (rec_0EB0 & 0x2) != 0
[0EC9]     GUARD (rec_097C & 0x2) == 0
[0ECF]     GUARD rec_0470 > 0
[0ED6]     ENDIF
[0ED7]     SAY "Commander, Daddy Gluxx talked about SLIM GELATI... Ask him for more information..."  '[skip 1]
[0EF7]     vbio -= 3
[0EFE]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0F22]     POKE [0x0EB4] = 0
  END
[0F26]   BLOCK (exit -> @0F93)
[0F2A]     AWAIT presentation
[0F2B]     GUARD active_actor == Honk.talk (related 40)
[0F30]     GUARD vbio > 2
[0F37]     GUARD rec_0332 == 65535
[0F3C]     GUARD rec_0860 == 0
[0F43]     GUARD rec_0842 == 3848
[0F48]     GUARD (rec_09E8 & 0x2) != 0
[0F4D]     ENDIF
[0F4E]     SAY "Commander, get back to the planet Moskito..."  '[skip 1]
[0F64]     vbio -= 3
[0F6B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0F8F]     POKE [0x0F27] = 0
  END
[0F93]   BLOCK (exit -> @0FF8)
[0F97]     AWAIT presentation
[0F98]     GUARD active_actor == Honk.talk (related 40)
[0F9D]     GUARD vbio > 2
[0FA4]     GUARD rec_01E8 == 0
[0FAB]     GUARD (rec_09E8 & 0x2) != 0
[0FB0]     ENDIF
[0FB1]     SAY "Commander, you haven't seen Izwalito in a while..."  '[skip 1]
[0FC9]     vbio -= 3
[0FD0]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0FF4]     POKE [0x0F94] = 0
  END
[0FF8]   BLOCK (exit -> @1060)
[0FFC]     AWAIT presentation
[0FFD]     GUARD active_actor == Honk.talk (related 40)
[1002]     GUARD vbio > 2
[1009]     GUARD rec_06B0 > 2
[1010]     GUARD NOT rec_1030 == 1658
[1016]     ENDIF
[1017]     SAY "Commander, SCRUTER MAC on Mastachok loves perfume from VENUSIA..."  '[skip 1]
[1031]     vbio -= 3
[1038]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[105C]     POKE [0x0FF9] = 0
  END
[1060]   BLOCK (exit -> @10C8)
[1064]     AWAIT presentation
[1065]     GUARD active_actor == Honk.talk (related 40)
[106A]     GUARD vbio > 2
[1071]     GUARD PP1 == 1
[1078]     GUARD NOT rec_1018 == 40
[107E]     ENDIF
[107F]     SAY "Commander, on VENUSIA, Bug Deluxe has lots of creds..."  '[skip 1]
[1099]     vbio -= 3
[10A0]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[10C4]     POKE [0x1061] = 0
  END
[10C8]   BLOCK (exit -> @1130)
[10CC]     AWAIT presentation
[10CD]     GUARD active_actor == Honk.talk (related 40)
[10D2]     GUARD vbio > 2
[10D9]     GUARD (rec_09E8 & 0x2) == 0
[10DF]     GUARD rec_02A2 == 3932
[10E4]     ENDIF
[10E5]     SAY "Commander, don't forget poor Morning Oil on the Ark ..."  '[skip 1]
[1101]     vbio -= 3
[1108]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[112C]     POKE [0x10C9] = 0
  END
[1130]   BLOCK (exit -> @11A3)
[1134]     AWAIT presentation
[1135]     GUARD active_actor == Honk.talk (related 40)
[113A]     GUARD vbio > 2
[1141]     GUARD (rec_097C & 0x2) != 0
[1146]     GUARD eka == 0
[114D]     GUARD rec_11B0 == 1514
[1152]     GUARD rec_0620 > 0
[1159]     ENDIF
[115A]     SAY "Commander, talk to Doctor Otto Von Smile about TRANSPLANT..."  '[skip 1]
[1174]     vbio -= 3
[117B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[119F]     POKE [0x1131] = 0
  END
[11A3]   BLOCK (exit -> @1347)
[11A7]     AWAIT presentation
[11A8]     GUARD active_actor == Honk.talk (related 40)
[11AD]     ENDIF
[11AE]     SAY "..."
[11B8]     SAY "What do you want Commander ? word_65535 talk remember bye_bye"
[11D6]     IF-BLOCK (exit -> @11E2)
[11D9]       GUARD concept == "remember"
[11DC]       ENDIF
[11DD]       POKE [0x1348] = 1
[11E1]       CLEAR concept_alt
    END
[11E2]     IF-BLOCK (exit -> @11EF)
[11E5]       GUARD concept == "talk"
[11E8]       ENDIF
[11E9]       rec_08B8 = 2902
[11EE]       CLEAR concept_alt
    END
[11EF]     IF-BLOCK (exit -> @121E)
[11F2]       GUARD concept == "bye_bye"
[11F5]       ENDIF
[11F6]       SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[121A]       CLEAR concept_alt
[121B]       END PRESENTATION Honk.talk
    END
[121E]     IF-BLOCK (exit -> @12A5)
[1221]       GUARD scr > 5
[1228]       ENDIF
[1229]       SAY "CHEAT MODE..."  '[skip 1]
[1235]       rec_08B8 = 2929
[123A]       SAY "Choose your script, Commander word_65535 3 4 5"
[1254]       IF-BLOCK (exit -> @126F)
[1257]         GUARD concept == "3"
[125A]         ENDIF
[125B]         SAY "Script 3 selected..."  '[skip 3]
[1269]         RUN PROFILE 2
[126B]         CLEAR concept_alt
[126C]         END PRESENTATION Honk.talk
      END
[126F]       IF-BLOCK (exit -> @128A)
[1272]         GUARD concept == "4"
[1275]         ENDIF
[1276]         SAY "Script 4 selected..."  '[skip 3]
[1284]         RUN PROFILE 3
[1286]         CLEAR concept_alt
[1287]         END PRESENTATION Honk.talk
      END
[128A]       IF-BLOCK (exit -> @12A5)
[128D]         GUARD concept == "5"
[1290]         ENDIF
[1291]         SAY "Script 5 selected..."  '[skip 3]
[129F]         RUN PROFILE 4
[12A1]         CLEAR concept_alt
[12A2]         END PRESENTATION Honk.talk
      END
    END
[12A5]     IF-BLOCK (exit -> @12BF)
[12A8]       GUARD concept == "bye_bye"
[12AB]       ENDIF
[12AC]       SAY "Bye bye , Commander..."  '[skip 1]
[12BC]       END PRESENTATION Honk.talk
    END
[12BF]     IF-BLOCK (exit -> @12FE)
[12C2]       GUARD state[15] == 0
[12C4]       ENDIF
[12C5]       SAY "Pardon me ,Commander , but I have work to do ... I have to quadrify the multiplexers ... See you !"  '[skip 2]
[12F7]       state[15] = 65535
[12FB]       END PRESENTATION Honk.talk
    END
[12FE]     IF-BLOCK (exit -> @1347)
[1301]       GUARD rec_0590 > 2
[1308]       GUARD NOT rec_1048 == 65535
[130E]       GUARD NOT rec_1018 == 40
[1314]       GUARD ti == 1
[131B]       ENDIF
[131C]       SAY "Commander, I just forged a CRED ! Take a look in the Cryobox ..."  '[skip 1]
[1340]       OP_CD CD 94 05 04 10 28 00
    END
  END
[1347]   GOTO @1C4C
[134B]   GUARD active_actor == Honk.talk (related 40)
[1350]   GUARD (rec_0D36 & 0x2) != 0
[1355]   ENDIF
[1356]   IF-BLOCK (exit -> @13BD)
[1359]     GUARD trak19 == 1
[1360]     ENDIF
[1361]     SAY "Well , we woke Cap'n Bob in his Cryobox . He told us a bunch of stuff and pointed out my deficiencies ..."
[1397]     SAY "Then we fearlessly entered a black hole and flew 500,000 years into the past ..."
  END
[13BD]   SAY "We got to a zone with heavy traffic buildup , where a SCRUTER invited us out ..."
[13E7]   SAY "You agreed with my evaluation of the situation and we bravely retreated ..."
[1409]   IF-BLOCK (exit -> @144A)
[140C]     GUARD (rec_0D36 & 0x2) != 0
[1411]     ENDIF
[1412]     SAY "I immediately detected the planet Corpo . You won't forget to mention that fact to Cap'n Bob , will you , Commander ? ..."
  END
[144A]   IF-BLOCK (exit -> @147D)
[144D]     GUARD trak2 == 1
[1454]     ENDIF
[1455]     SAY "We first met Izwalito on the planet Corpo . He was one hungry little guy ..."
  END
[147D]   IF-BLOCK (exit -> @14A4)
[1480]     GUARD trak2b == 1
[1487]     ENDIF
[1488]     SAY "He told us where to locate the planet MAGNUS ..."
  END
[14A4]   IF-BLOCK (exit -> @14F5)
[14A7]     GUARD trak3 == 1
[14AE]     ENDIF
[14AF]     SAY "We came back to see him and he asked us to get some frozen murffalo meat from the planet Moskito"
[14DF]     SAY "And he gave us a CRED ..."
  END
[14F5]   IF-BLOCK (exit -> @1554)
[14F8]     GUARD trak4 == 1
[14FF]     ENDIF
[1500]     SAY "You made an error of judgment , Commander , when you purchased a lump of murffalo on Venusia"
[152C]     SAY "That wasn't what Izwalito had requested , was it ? He experienced feelings of impatience ..."
  END
[1554]   IF-BLOCK (exit -> @158B)
[1557]     GUARD trak5 == 1
[155E]     ENDIF
[155F]     SAY "We got a call from BOSSANOVA , Izwalito's love partner . She was unhappy about the situation ..."
  END
[158B]   IF-BLOCK (exit -> @15E0)
[158E]     GUARD trak6 == 1
[1595]     ENDIF
[1596]     SAY "Then we bought three tons of murffalo from Bronko on the planet Moskito ..."
[15BA]     SAY "And we brought it back to Izwalito who gave it to his grateful people ..."
  END
[15E0]   IF-BLOCK (exit -> @1611)
[15E3]     GUARD C0 == 1
[15EA]     ENDIF
[15EB]     SAY "You may recall we received a mysterious message from far away in the galaxy ..."
  END
[1611]   IF-BLOCK (exit -> @1648)
[1614]     GUARD C0 == 2
[161B]     ENDIF
[161C]     SAY "I easily decoded the first word of the coded message from far away . The word was coded"
  END
[1648]   IF-BLOCK (exit -> @1691)
[164B]     GUARD C0 == 3
[1652]     ENDIF
[1653]     SAY "I quickly decoded the entire message from far away . It said CODED MESSAGE ... You will tell Cap'n Bob , won't you , Commander ? ..."
  END
[1691]   IF-BLOCK (exit -> @16F8)
[1694]     GUARD trak1 == 1
[169B]     ENDIF
[169C]     SAY "We got a pirate message from another player who hooked up to your computer , remember ? ..."
[16C8]     SAY "That other player was impressively talented ... He must have used a neural net to link up with us ..."
  END
[16F8]   IF-BLOCK (exit -> @1765)
[16FB]     GUARD trak7 == 1
[1702]     ENDIF
[1703]     SAY "Izwalito arranged for us to meet Hom , a buddy of his , in the village of HITA ..."
[1731]     SAY "Hom is a fun person , but strictly no longer at this address mindwise . The poor guy's brain is tubular ..."
  END
[1765]   IF-BLOCK (exit -> @1792)
[1768]     GUARD trak8 == 1
[176F]     ENDIF
[1770]     SAY "We teleported Bronko into the cryobox and he gives the planet Ekatomb ..."
  END
[1792]   IF-BLOCK (exit -> @17E9)
[1795]     GUARD trak9 == 1
[179C]     ENDIF
[179D]     SAY "Ha! Ha! I chuckled when I recall that poor Croolis who replaced Bronko as murffalo sales-being on Moskito ..."
[17CB]     SAY "He's called EMASCULATOR , EVISCERATOR's sworn enemy. What a character ..."
  END
[17E9]   IF-BLOCK (exit -> @182C)
[17EC]     GUARD trak10 == 1
[17F3]     ENDIF
[17F4]     SAY "We visited Migrator at the airport on Moskito"
[180C]     SAY "He talked about his music . We had a fine time ..."
  END
[182C]   IF-BLOCK (exit -> @18A7)
[182F]     GUARD trak11 == 1
[1836]     ENDIF
[1837]     SAY "We located TINA BURNER who was singing at the PURPLE HAZE on the planet Eden ..."
[185F]     SAY "We teleported her to Migrator on Moskito . He was overjoyed ..."
[187F]     SAY "And you lost your chance of finding true love , Commander ... Hee! Hee! Hee! ..."
  END
[18A7]   IF-BLOCK (exit -> @18F4)
[18AA]     GUARD trak12 == 1
[18B1]     ENDIF
[18B2]     SAY "A little later , Migrator announced his marriage to the luscious Tina . Poor guy ..."
[18DA]     SAY "He asked to give him the wedding ring ..."
  END
[18F4]   IF-BLOCK (exit -> @1961)
[18F7]     GUARD trak13 == 1
[18FE]     ENDIF
[18FF]     SAY "We came across Morning Oil on planet Magnus . Boy , was he in a lousy condition ..."
[192B]     SAY "I instantaneously noticed how flat his batteries were ... That's something you'll mention to Cap'n Bob , won't you , Commander ? ..."
  END
[1961]   IF-BLOCK (exit -> @19CE)
[1964]     GUARD trak14 == 1
[196B]     ENDIF
[196C]     SAY "We bought some batteries for poor old Morning Oil at VENUSIA supramarket ..."
[198E]     SAY "We put the new batteries in Morning Oil , and he started working again ..."
[19B4]     SAY "So we teleported him aboard to study him ..."
  END
[19CE]   IF-BLOCK (exit -> @19FF)
[19D1]     GUARD trak15 == 1
[19D8]     ENDIF
[19D9]     SAY "I was obliged to switch Morning Oil off , to check out his memory ..."
  END
[19FF]   IF-BLOCK (exit -> @1A36)
[1A02]     GUARD trak16 == 1
[1A09]     ENDIF
[1A0A]     SAY "I speedily discovered in Morning Oil's memory that the story about the Croolis war treasure was true ..."
  END
[1A36]   IF-BLOCK (exit -> @1A7B)
[1A39]     GUARD trak17 == 1
[1A40]     ENDIF
[1A41]     SAY "You interrogated Morning Oil and forced him to reveal the existence of the planet MASTACHOK , where his ex-master EVISCERATOR was to be found ."
  END
[1A7B]   IF-BLOCK (exit -> @1AD0)
[1A7E]     GUARD trak18 == 1
[1A85]     ENDIF
[1A86]     SAY "Then , uh , then we got a message from Hektor's wife . Hektor piloted that drone you destroyed , remember ? ..."
[1ABC]     SAY "That was a painful moment ..."
  END
[1AD0]   IF-BLOCK (exit -> @1AF5)
[1AD3]     GUARD trak20 == 1
[1ADA]     ENDIF
[1ADB]     SAY "Morning Oil told us about BIONIUM and ODDLAND CYBERSPACE"
  END
[1AF5]   IF-BLOCK (exit -> @1B16)
[1AF8]     GUARD trak21 == 1
[1AFF]     ENDIF
[1B00]     SAY "We met Yoko on the planet RONDO"
  END
[1B16]   IF-BLOCK (exit -> @1B3B)
[1B19]     GUARD trak22 == 1
[1B20]     ENDIF
[1B21]     SAY "Mister Maxxon explained about his telescope with no lens"
  END
[1B3B]   IF-BLOCK (exit -> @1B62)
[1B3E]     GUARD trak24 == 1
[1B45]     ENDIF
[1B46]     SAY "We ran into the GLUXX family on the planet Ekatomb"
  END
[1B62]   IF-BLOCK (exit -> @1B91)
[1B65]     GUARD trak25 == 1
[1B6C]     ENDIF
[1B6D]     SAY "Father GLUXX mentioned the planet Erazor where doctor Otto Von Smile has his clinic"
  END
[1B91]   IF-BLOCK (exit -> @1BB4)
[1B94]     GUARD trak26 == 1
[1B9B]     ENDIF
[1B9C]     SAY "We encountered the unutterably horrible doctor on Erazor"
  END
[1BB4]   IF-BLOCK (exit -> @1BE9)
[1BB7]     GUARD trak27 == 1
[1BBE]     ENDIF
[1BBF]     SAY "He gave us a telescope lens and we gave him the Gluxx's address on the planet Ekatomb"
  END
[1BE9]   IF-BLOCK (exit -> @1C2C)
[1BEC]     GUARD trak23 == 1
[1BF3]     ENDIF
[1BF4]     SAY "We returned to the planet Rondo"
[1C08]     SAY "We delivered Otto Von Smile's lens to Maxxon who seemed less than happy ..."
  END
[1C2C]   SAY "You need anything else ? Just say the word ..."  '[skip 1]
[1C48]   POKE [0x1348] = 0
[1C4C]   BLOCK (exit -> @1D73)
[1C50]     AWAIT gameflag_274F
[1C51]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1C56]     GUARD BO == 0
[1C5D]     ENDIF
[1C5E]     SAY "HONK ! You worthless heap of wires ... Are you working ?"  '[voice 2]
[1C7E]     SAY "Yes sir, Cap'n Bob sir ! ... Just getting the multiplexers toned up ..."
[1CA2]     SAY "What do you want to know , Commander ?"  '[voice 3, skip 1]
[1CBC]     state[1] = 50
[1CC0]     IF-BLOCK (exit -> @1D14)
[1CC3]       GUARD state[1] == 0
[1CC5]       ENDIF
[1CC6]       SAY "I feel like a dog I micro-waved by mistake one day ... I have to cryonize ..."  '[voice 6]
[1CF0]       SAY "Ahhhh !!!"  '[voice 3]
[1CFC]       SAY "stop"  '[skip 3]
[1D06]       BO = 2
[1D0D]       state[1] = 65535
[1D11]       END PRESENTATION Bob_Morlock.talk
    END
[1D14]     SAY "I feel weak , Commander ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[1D34]     adieu = 1
[1D3B]     IF-BLOCK (exit -> @1D73)
[1D3E]       GUARD adieu == 1
[1D45]       ENDIF
[1D46]       SAY "Ah, sleep ..."
[1D54]       SAY "stop"  '[skip 4]
[1D5E]       adieu = 0
[1D65]       BO = 2
[1D6C]       state[1] = 65535
[1D70]       END PRESENTATION Bob_Morlock.talk
    END
  END
[1D73]   BLOCK (exit -> @1F71)
[1D77]     AWAIT gameflag_274F
[1D78]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1D7D]     GUARD BO == 2
[1D84]     ENDIF
[1D85]     SAY "My bones feel like they've been through a shredder ... HONK! Miserable machine ! Give me a shot of ASPIROX !!!"  '[voice 4, skip 1]
[1DB7]     adieu = 0
[1DBE]     SAY "Yes sir, Cap'n Bob . I'm locating the seringe even as I utter these words ..."
[1DE6]     SAY "Be careful ... Find a spot with some flesh on it . The last time , you stuck the needle straight into a bone !!!"  '[voice 5]
[1E20]     SAY "Yes ! Yes ! Cap'n Bob , I'll be careful , I'll shoot another buttock, I'll choose the third one ..."
[1E52]     SAY "I'm listening , Commander ..."  '[voice 3, skip 1]
[1E64]     state[1] = 60
[1E68]     IF-BLOCK (exit -> @1F0A)
[1E6B]       GUARD state[1] == 0
[1E6D]       ENDIF
[1E6E]       SAY "AAARGH !!! HONK YOU STABBED ME IN THE BONE AGAIN ... I told you not to do that ..."  '[voice 5]
[1E9C]       SAY "Sorry about that, Cap'n Bob . The needle slipped ..."
[1EB8]       SAY "I feel like the frog who stood up to the truck , Commander ! I have to cryonize ..."  '[voice 6]
[1EE6]       SAY "Ahhhh !!!"  '[voice 3]
[1EF2]       SAY "stop"  '[skip 3]
[1EFC]       BO = 3
[1F03]       state[1] = 65535
[1F07]       END PRESENTATION Bob_Morlock.talk
    END
[1F0A]     SAY "I feel awful , Commander ... You're tiring me out ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[1F34]     adieu = 1
[1F3B]     IF-BLOCK (exit -> @1F71)
[1F3E]       GUARD adieu == 1
[1F45]       ENDIF
[1F46]       SAY "Sleep ..."
[1F52]       SAY "stop"  '[skip 4]
[1F5C]       adieu = 0
[1F63]       BO = 3
[1F6A]       state[1] = 65535
[1F6E]       END PRESENTATION Bob_Morlock.talk
    END
  END
[1F71]   BLOCK (exit -> @212C)
[1F75]     AWAIT gameflag_274F
[1F76]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1F7B]     GUARD BO == 3
[1F82]     ENDIF
[1F83]     SAY "I'm thirsty ... HONK! You reject fridge component ! Give me a glass of recycled water !!!"  '[voice 4]
[1FAD]     SAY "Yes sir, Cap'n Bob ! Just locating a clean glass , beaker , mug , cup or other drinking vessel ..."
[1FDF]     SAY "Be careful ... I said recycled water , not liquid coolant like the last time !!!"  '[voice 5]
[2007]     SAY "Trust me , Cap'n Bob ! I've learned My lesson . Yes sir ! I'm going to taste it to make sure ..."
[203D]     SAY "Don't do that , you moronic box of longwinded incompetence ! You'll short-circuit !"  '[voice 3]
[2061]     SAY "I'm listening, Commander ..."  '[voice 3]
[2071]     state[1] = 120
[2075]     IF-BLOCK (exit -> @20B9)
[2078]       GUARD state[1] == 0
[207A]       ENDIF
[207B]       SAY "I'm too hot ... I need to cryonize ..."  '[voice 6]
[2095]       SAY "AAAAAAhhhh !!!"  '[voice 3]
[20A1]       SAY "stop"  '[skip 3]
[20AB]       BO = 4
[20B2]       state[1] = 65535
[20B6]       END PRESENTATION Bob_Morlock.talk
    END
[20B9]     SAY "I feel weak as that kitten I accidentally converted to anti-matter , Commander ... Let me get some sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[20ED]     adieu = 1
[20F4]     IF-BLOCK (exit -> @212C)
[20F7]       GUARD adieu == 1
[20FE]       ENDIF
[20FF]       SAY "Ah, sleep ..."
[210D]       SAY "stop"  '[skip 4]
[2117]       adieu = 0
[211E]       BO = 4
[2125]       state[1] = 65535
[2129]       END PRESENTATION Bob_Morlock.talk
    END
  END
[212C]   BLOCK (exit -> @22DA)
[2130]     AWAIT gameflag_274F
[2131]     GUARD active_actor == Bob_Morlock.talk (related 40)
[2136]     GUARD BO == 4
[213D]     ENDIF
[213E]     IF-BLOCK (exit -> @221B)
[2141]       GUARD rec_0080 == 6
[2148]       ENDIF
[2149]       SAY "I'm hungry ... HONK! You no-good collection of second-hand wiring ! Where's the menu !!!"  '[voice 4]
[216F]       SAY "Coming right up , Cap'n Bob ! Your dinner is heating up in the engine room ..."
[2199]       SAY "Be careful this time ... Hot food does not have to be curled up, black and smoking !!!"  '[voice 5]
[21C5]       SAY "Not smoking... You got it , Cap'n Bob , sir ... Would you care to browse through the MENU ?"
[21F5]       SAY "WHAT ?? GLOK AGAIN !!! I'd like to see you curled up and smoking ..."  '[voice 6]
    END
[221B]     SAY "What do you want to know Commander ..."  '[voice 6, skip 1]
[2233]     state[1] = 90
[2237]     IF-BLOCK (exit -> @226C)
[223A]       GUARD state[1] == 0
[223C]       ENDIF
[223D]       SAY "I need to cryonize ..."  '[voice 6]
[224F]       SAY "AAAAAAAAAAAAhhhh !!!"  '[voice 3]
[225B]       SAY "stop"  '[skip 2]
[2265]       state[1] = 65535
[2269]       END PRESENTATION Bob_Morlock.talk
    END
[226C]     SAY "I feel weak as that lamb I inadvertently dropped in my shark-pool one day , Commander ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[22A2]     adieu = 1
[22A9]     IF-BLOCK (exit -> @22DA)
[22AC]       GUARD adieu == 1
[22B3]       ENDIF
[22B4]       SAY "Ah, sleep ..."
[22C2]       SAY "stop"  '[skip 3]
[22CC]       adieu = 0
[22D3]       state[1] = 65535
[22D7]       END PRESENTATION Bob_Morlock.talk
    END
  END
[22DA]   BLOCK (exit -> @23D2)
[22DE]     AWAIT gameflag_274F
[22DF]     GUARD active_actor == Bob_Morlock.talk (related 40)
[22E4]     GUARD emplo == 1
[22EB]     GUARD emp == 0
[22F2]     ENDIF
[22F3]     SAY "While you're awake, Cap'n Bob , sir ... I have a request to make ..."
[2319]     SAY "Olga and me would like a raise . A few megawatts ..."
[2339]     SAY "WHAT ... SPEAK LOUDER ... I'M GETTING DEAFER AND DEAFER ..."  '[voice 5]
[2357]     SAY "WE WANT MORE MEGAWATTS !!!"
[2369]     SAY "I feel like that canary I mistook for an oven-ready turkey , Commander . Haaa !!! My heart ...."  '[voice 4]
[2397]     SAY "Heart !! ... He means his power generator ... Those guys ... Just mention a couple of lousy megawatts !"  '[skip 2]
[23C7]     emp = 1
[23CE]     POKE [0x22DB] = 0
  END
[23D2]   BLOCK (exit -> @25DE)
[23D6]     AWAIT gameflag_274F
[23D7]     GUARD active_actor == Bob_Morlock.talk (related 40)
[23DC]     GUARD reve == 1
[23E3]     GUARD revelat == 0
[23EA]     ENDIF
[23EB]     SAY "You want to know an unbearable truth , Commander ?"  '[voice 7]
[2407]     SAY "HONK! Switch off for ten seconds !!!"  '[voice 6]
[241D]     SAY "But , Cap'n Bob !! ..."
[2431]     SAY "YOU HEARD ME ... SWITCH YOURSELF OFF !!!"  '[voice 5]
[2449]     SAY "Yes sir , Cap'n Bob . If you say so ..."
[2467]     SAY "KRUIIIIK !!! AAAaaaaaaaaaaaaaaaaaaaaa !!!"
[2477]     SAY "COMMANDER, YOU ARE ME ..."  '[voice 5]
[2489]     SAY "WE ARE THE SAME PERSON AT TWO DIFFERENT AGES ..."  '[voice 6]
[24A5]     SAY "YOU ARE MUCH MORE THAN MY SON ..."  '[voice 4]
[24BD]     SAY "We are the same person ... I am the first being to have created itself"  '[voice 5]
[24E3]     SAY "And who , thanks to space-time distortion , can watch itself re-live : YOU ARE BOB , COMMANDER ..."  '[voice 4]
[2511]     SAY "I am what you will become hundreds of thousands of years from now ..."  '[voice 2]
[2535]     SAY "OK Honk , you can switch on now ..."  '[voice 6]
[254F]     SAY "KROIIIIkkk !!! -&KRUIIIIkkk !!! You guys should know I have a right to stay informed . That's what being ONBOARD COMPUTER is all about , right ?"
[258D]     SAY "Was Olga switched off too ???"  '[skip 1]
[25A1]     plus = 1
[25A8]     SAY "Stop snivelling and get to work , you jumped up little fuse box ..."  '[voice 5, skip 3]
[25CC]     revelat = 1
[25D3]     reve = 0
[25DA]     POKE [0x23D3] = 0
  END
[25DE]   BLOCK (exit -> @272F)
[25E2]     AWAIT gameflag_274F
[25E3]     GUARD vol == 1
[25EA]     GUARD active_actor == Bob_Morlock.talk (related 40)
[25EF]     ENDIF
[25F0]     SAY "Cap'n Bob, I have to tell you the Commander has behaved very badly ..."
[2614]     SAY "He stole money from a defenceless Izwal of planet Corpo and then he spent it ..."
[263C]     SAY "I'm sorry to say it's true ..."
[2652]     SAY "Miserable machine !!! WHO DO YOU THINK YOU ARE ? He did whatever the situation demanded ..."  '[voice 5]
[267C]     SAY "True , Commander ? word_65535 yes no"  '[voice 4]
[2694]     IF-BLOCK (exit -> @26CA)
[2697]       GUARD concept == "yes"
[269A]       ENDIF
[269B]       SAY "Well ... If I remember , there wasn't a lot of choices ..."  '[voice 4]
[26BD]       SAY "Exactly ..."  '[skip 1]
[26C9]       CLEAR concept_alt
    END
[26CA]     IF-BLOCK (exit -> @272F)
[26CD]       GUARD concept == "no"
[26D0]       ENDIF
[26D1]       SAY "You see , you pathetic excuse for a machine : YOU'RE A LIAR !!! I knew it all along ..."  '[voice 5]
[2701]       SAY "But Cap'n Bob ... I swear it's true ..."
[271B]       SAY "SILENCE !!!"  '[voice 2, skip 2]
[2727]       vol = 0
[272E]       CLEAR concept_alt
    END
  END
[272F]   BLOCK (exit -> @2744)
[2733]     ENDIF
[2734]     state[3] = 10
[2738]     state[4] = 200
[273C]     OP_B7 B7 E0 06 02
[2740]     POKE [0x2730] = 0
  END
[2744]   BLOCK (exit -> @2758)
[2748]     GUARD state[3] == 0
[274A]     ENDIF
[274B]     OP_C3 C3 FC 06 28 00
[2750]     POKE [0x2759] = 1
[2754]     POKE [0x2745] = 0
  END
[2758]   GOTO @27CF
[275C]   AWAIT presentation
[275D]   GUARD active_actor == Scruter_K.talk (related 40)
[2762]   ENDIF
[2763]   SAY ""  '[skip 1]
[276B]   SS = 1
[2772]   SAY ""  '[skip 1]
[277A]   SS = 2
[2781]   SAY ""  '[skip 1]
[2789]   SS = 3
[2790]   SAY ""  '[skip 1]
[2798]   SS = 4
[279F]   SAY ""  '[skip 1]
[27A7]   SS = 5
[27AE]   IF-BLOCK (exit -> @27BC)
[27B1]     GUARD SS == 0
[27B8]     ENDIF
[27B9]     GOTO @2763
  END
[27BC]   IF-BLOCK (exit -> @27CF)
[27BF]     GUARD SS > 0
[27C6]     ENDIF
[27C7]     POKE [0x2759] = 0
[27CB]     POKE [0x27D0] = 1
  END
[27CF]   GOTO @2DBE
[27D3]   AWAIT presentation
[27D4]   GUARD active_actor == Scruter_K.talk (related 40)
[27D9]   ENDIF
[27DA]   IF-BLOCK (exit -> @2904)
[27DD]     GUARD SS == 1
[27E4]     ENDIF
[27E5]     SAY "MESSAGE RADIO:"
[27F1]     SAY "Calling garbage scow :"
[2801]     SAY "This is SCRUT robot "HYDRAULIC JACK OF TITANIUM ALLOY", squadron commander ..."
[2821]     SAY "You are in a forbidden zone ... THIS IS A WARNING ..."
[2841]     SAY "YOU ARE SCHEDULED FOR VAPORIZATION IN TWO MINUTES , TEN SECONDS ..."
[2861]     SAY "MAKE THAT NINE SECONDS ..."
[2873]     SAY "EIGHT SECONDS ..."
[2881]     SAY "SEVEN ..."
[288D]     SAY "Get that garbage wagon out of here ! Scram ! Otherwise my hi-powered laser weapons will spell death !!!..."
[28BB]     SAY "REPORT FROM THE ARK'S ONBOARD COMPUTER FACILITY , HONK :"
[28D7]     SAY "Commander , He means it ... the whole zone's filled with guys like him : RED ALERT"
[2901]     GOTO @2D40
  END
[2904]   IF-BLOCK (exit -> @2A14)
[2907]     GUARD SS == 2
[290E]     ENDIF
[290F]     SAY "MESSAGE RADIO:"
[291B]     SAY "LISTEN UP"
[2927]     SAY "This is SCRUT robot "NONSTICK HEART" , area sub-director , calling garbage can ..."
[294B]     SAY "THIS IS A WARNING ... YOU ARE IN A FORBIDDEN ZONE"
[2969]     SAY "PREPARE FOR PAINFUL OBLITERIZATION IN TEN SECONDS ..."
[2981]     SAY "NINE SECONDS ..."
[298F]     SAY "TWELVE SECONDS ..."
[299D]     SAY "UH , SORRY ABOUT THAT ..."
[29B1]     SAY "EIGHT ..."
[29BD]     SAY "LEAVE NOW OR MEET ROSEBUD ? MY LASER-PULPER"
[29D5]     SAY "Honk reports in at this time :"
[29EB]     SAY "I advise we comply fast , Commander. He's liable to skip four and three ..."
[2A11]     GOTO @2D40
  END
[2A14]   IF-BLOCK (exit -> @2AF4)
[2A17]     GUARD SS == 3
[2A1E]     ENDIF
[2A1F]     SAY "MESSAGE RADIO:"
[2A2B]     SAY "ATTENTION ..."
[2A37]     SAY "THIS IS SCRUT TROOPER "TUNG STEN" , TO UNKNOWN SHIP-TYPE THING ..."
[2A57]     SAY "I HAVE THE FOLLOWING WARNING : THIS SECTOR IS FORBIDDEN TO Y-O-U !!!"
[2A79]     SAY "WE WILL MANGLE YOUR BODY ORGANS , MELT DOWN YOUR MELTABLE PARTS , AND ALSO VITRIFY YOU IF YOU DON'T GO AWAY"
[2AAD]     SAY "THREE SECONDS ..."
[2ABB]     SAY "TWO SECONDS ..."
[2AC9]     SAY "Honk reports , in the nick of time :"
[2AE3]     SAY "Red Alert !"
[2AF1]     GOTO @2D40
  END
[2AF4]   IF-BLOCK (exit -> @2C2C)
[2AF7]     GUARD SS == 4
[2AFE]     ENDIF
[2AFF]     SAY "MESSAGE RADIO:"
[2B0B]     SAY "OKAY WHOEVER YOU ARE"
[2B1B]     SAY "This is low-ranking SCRUT robot "STAINLESS STEEL CASING" , CALLING FLYING JUNK HEAP ..."
[2B3F]     SAY "MOVE OUT OF THIS FORBIDDEN SECTOR ... AND MAKE IT SNAPPY , BUDDY BOY !!!"
[2B65]     SAY "Otherwise you get STOMPED ON ..."
[2B79]     SAY "YOU HAVE TEN SECONDS BEFORE ANNIHILATION SETS IN ..."
[2B93]     SAY "NINE SECONDS ..."
[2BA1]     SAY "TWELVE SECONDS ..."
[2BAF]     SAY "OKAY OKAY , WISE GUY !"
[2BC3]     SAY "YOU DO THE COUNTING ..."
[2BD5]     SAY "CRUIIIIK!"
[2BDF]     SAY "Report from Honk , onboard computing entity :"
[2BF7]     SAY "It's true he's no shining beacon of brainpower , Commander . And yet I feel moved by his obvious sincerity ..."
[2C29]     GOTO @2D40
  END
[2C2C]   IF-BLOCK (exit -> @2D40)
[2C2F]     GUARD SS == 5
[2C36]     ENDIF
[2C37]     SAY "MESSAGE RADIO:"
[2C43]     SAY "LISTEN UP"
[2C4F]     SAY "THIS IS SCRUT ROBOT "TENSILE SHEET METAL" , AREA CHIEF , TO UNKNOWN VESSEL ..."
[2C75]     SAY "THIS IS A PL7K stroke 38B WARNING : YOU HAVE TWO MINUTES TO EVACUATE THE SECTOR"
[2C9D]     SAY "FAILURE TO COMPLY WITH ABOVE PL7K stroke 38B WARNING WILL RESULT IN A 1010 ..."
[2CC3]     SAY "YOURS SINCERELY ,"
[2CD1]     SAY "SIGNED : SCRUTER "TENSILE SHEET METAL" , AREA CHIEF WITH ADDED LASER POWER"
[2CF3]     SAY "Heeeere's Honky ! ..."
[2D03]     SAY "That stuff about 1010 ? That's their way of saying meet your maker . There's a big bunch of them SCRUT ships around here ..."
[2D3D]     GOTO @2D40
  END
[2D40]   SAY "Whaddaya say we just do like the robot says before the laser pulpers kick in ..."  '[skip 1]
[2D68]   rec_06C4 &= !0x2
[2D6E]   SAY "CLICK ON THE RED BUTTON ON THE MAP , SELECT A PLANET AND PUSH THE CONTROL STICK ..."  '[skip 1]
[2D9A]   A0 = 1
[2DA1]   SAY "stop"  '[skip 5]
[2DAB]   POKE [0x27D0] = 0
[2DAF]   POKE [0x2745] = 0
[2DB3]   state[8] = 10
[2DB7]   POKE [0x27D0] = 0
[2DBB]   END PRESENTATION Scruter_K.talk
[2DBE]   BLOCK (exit -> @2DE3)
[2DC2]     GUARD rec_0590 < 2
[2DC9]     GUARD rec_0740 < 2
[2DD0]     GUARD state[8] == 0
[2DD2]     ENDIF
[2DD3]     OP_C3 C3 FC 06 28 00
[2DD8]     T1 = 1
[2DDF]     POKE [0x2DBF] = 0
  END
[2DE3]   BLOCK (exit -> @2F74)
[2DE7]     GUARD T1 == 1
[2DEE]     AWAIT presentation
[2DEF]     GUARD active_actor == Scruter_K.talk (related 40)
[2DF4]     ENDIF
[2DF5]     SAY "MESSAGE RADIO:"
[2E01]     SAY "This is SCRUT agent K , district director . Message to unknown vessel ..."
[2E25]     IF-BLOCK (exit -> @2E6E)
[2E28]       GUARD NOT rec_0F4E == 3380
[2E2E]       ENDIF
[2E2F]       SAY "You deaf or something ?"
[2E41]       SAY "Get out of here ... SWEAR ... INSULT ..."
[2E5B]       SAY "FINAL WARNING"  '[skip 1]
[2E67]       kill = 1
    END
[2E6E]     IF-BLOCK (exit -> @2F44)
[2E71]       GUARD rec_0F4E == 3380
[2E76]       ENDIF
[2E77]       SAY "Next time , you'll really get it ..."
[2E8F]       SAY "You won't have time to be sorry ..."
[2EA7]       SAY "Ha! Ha! Ha! Ha! ... ... ... ... ... ... ... Hurk ! Hurk ! Haurk !"
[2ED1]       SAY "REPORT FROM HONK:"
[2EDF]       SAY "Commander ! Commander ! We really fooled those dummies ... Ha! Ha! Ha! Ha!"
[2F03]       SAY "Hm ! Sorry about that ..."
[2F17]       IF-BLOCK (exit -> @2F44)
[2F1A]         GUARD A1 == 0
[2F21]         ENDIF
[2F22]         SAY "Click on the planet Corpo . The Orxx will be automatically ejected ..."
      END
    END
[2F44]     SAY "..."  '[skip 1]
[2F4E]     rec_06C4 &= !0x2
[2F54]     SAY "stop"  '[skip 5]
[2F5E]     POKE [0x2DE4] = 0
[2F62]     T1 = -1.value
[2F69]     POKE [0x2F75] = 1
[2F6D]     POKE [0x3043] = 1
[2F71]     END PRESENTATION Scruter_K.talk
  END
[2F74]   GOTO @2F93
[2F78]   GUARD kill == 1
[2F7F]   GUARD state[4] == 0
[2F81]   ENDIF
[2F82]   POKE [0x27D0] = 0
[2F86]   OP_C3 C3 FC 06 28 00
[2F8B]   POKE [0x2F94] = 1
[2F8F]   POKE [0x2F75] = 0
[2F93]   GOTO @3021
[2F97]   AWAIT presentation
[2F98]   GUARD active_actor == Scruter_K.talk (related 40)
[2F9D]   ENDIF
[2F9E]   SAY "MESSAGE RADIO:"
[2FAA]   SAY "SWEAR ... INSULT ... SWEAR ... ROARS ... INSULT ..."
[2FC6]   SAY "You're lucky we've been called to another sector ... INSULT ..."
[2FE4]   SAY "WE'LL MEET AGAIN ... YOU'LL SEE ..."
[2FFA]   SAY "HEAK ! HUURK !!! FILTHY INSULT ..."
[3010]   SAY "STOP"  '[skip 2]
[301A]   END PRESENTATION Scruter_K.talk
[301D]   POKE [0x2F94] = 0
[3021]   BLOCK (exit -> @3042)
[3025]     GUARD A0 == 1
[302C]     ENDIF
[302D]     rec_0D36 |= 0x2
[3032]     Korpo_WW = etat.value
[3039]     rec_0DA2 |= 0x2
[303E]     POKE [0x3022] = 0
  END
[3042]   GOTO @3062
[3046]   GUARD rec_0F4E == 3380
[304B]   ENDIF
[304C]   POKE [0x2745] = 0
[3050]   POKE [0x27D0] = 0
[3054]   POKE [0x2F75] = 0
[3058]   rec_06C4 &= !0x2
[305E]   POKE [0x3043] = 0
[3062]   BLOCK (exit -> @3081)
[3066]     AWAIT presentation
[3067]     GUARD CO == 0
[306E]     GUARD rec_0F4E == 3074
[3073]     ENDIF
[3074]     OP_C3 C3 B4 06 28 00
[3079]     POKE [0x3082] = 1
[307D]     POKE [0x3063] = 0
  END
[3081]   GOTO @313C
[3085]   AWAIT presentation
[3086]   GUARD active_actor == Scruter_Mac.talk (related 40)
[308B]   ENDIF
[308C]   SAY "MESSAGE RADIO:"  '[skip 1]
[3098]   rec_06C0 = 2929
[309D]   SAY "Beep beep bop bop bop beeep Tiiiii TATATA scraaak ..."
[30B9]   SAY "HONK REPORTING IN :"
[30C9]   SAY "Commander , that weird message you just heard came from a distant point in the galaxy . It's coded ..."
[30F9]   SAY "Excuse me while I figure out what vital information it contains ..."
[3119]   SAY "stop"  '[skip 5]
[3123]   POKE [0x313D] = 1
[3127]   rec_06B0 = 0
[312E]   CO = 1
[3135]   POKE [0x3082] = 0
[3139]   END PRESENTATION Scruter_Mac.talk
[313C]   GOTO @315B
[3140]   GUARD CO == 1
[3147]   AWAIT presentation
[3148]   GUARD rec_0F4E == 3818
[314D]   ENDIF
[314E]   OP_C3 C3 B4 06 28 00
[3153]   POKE [0x315C] = 1
[3157]   POKE [0x313D] = 0
[315B]   GOTO @3215
[315F]   AWAIT presentation
[3160]   GUARD active_actor == Scruter_Mac.talk (related 40)
[3165]   ENDIF
[3166]   SAY "MESSAGE RADIO:"
[3172]   SAY "Beep beep bop bop bop beeep ... Tut Tut Ts Ts Tsk Craaak ..."
[3196]   SAY "KRUIKK !"
[31A2]   SAY "HONK REPORTS :"
[31B0]   SAY "Commander , I decoded the first word . It says CODED . Funny , wouldn't you say ?"
[31DC]   SAY "CODED ... Well how about that !"
[31F2]   SAY "stop"  '[skip 5]
[31FC]   POKE [0x3216] = 1
[3200]   rec_06B0 = 0
[3207]   CO = 2
[320E]   POKE [0x315C] = 0
[3212]   END PRESENTATION Scruter_Mac.talk
[3215]   GOTO @3234
[3219]   AWAIT presentation
[321A]   GUARD rec_0F4E == 3380
[321F]   GUARD CO == 2
[3226]   ENDIF
[3227]   OP_C3 C3 B4 06 28 00
[322C]   POKE [0x3235] = 1
[3230]   POKE [0x3216] = 0
[3234]   BLOCK (exit -> @3352)
[3238]     AWAIT presentation
[3239]     GUARD active_actor == Scruter_Mac.talk (related 40)
[323E]     ENDIF
[323F]     SAY "MESSAGE RADIO:"
[324B]     SAY "Bee bee rla rla bop bop bop beeep ... Turlu ta ta ta tweeeeet ..."
[3271]     SAY "KRUIKK !"
[327D]     SAY "HONK SPEAKING :"
[328B]     SAY "Commander , I decoded the second word . It says MESSAGE ... ..."
[32AD]     SAY "Now , that gives us ... CODED MESSAGE:"EXXOS" ... That proves I was right , Commander ! The message is coded !"
[32E1]     SAY "I don't mind believing in coincidences, Commander , but that one's a biggie, right ?"
[3307]     SAY "You'll remember to tell ol' Buzzard Man Bob I cracked the code , won't you , Commander ?"
[3333]     SAY "stop"  '[skip 4]
[333D]     CO = 3
[3344]     rec_06B0 = 0
[334B]     POKE [0x3235] = 0
[334F]     END PRESENTATION Scruter_Mac.talk
  END
[3352]   BLOCK (exit -> @335F)
[3356]     ENDIF
[3357]     state[12] = 1200
[335B]     POKE [0x3353] = 0
  END
[335F]   BLOCK (exit -> @3375)
[3363]     ENDIF
[3364]     IF-BLOCK (exit -> @3375)
[3367]       GUARD state[12] == 0
[3369]       ENDIF
[336A]       ti += 1
[3371]       state[12] = 1200
    END
  END
[3375]   BLOCK (exit -> @338B)
[3379]     GUARD ti == 1
[3380]     ENDIF
[3381]     rec_00DC |= 0x2
[3386]     OP_C3 C3 14 01 28 00
  END
[338B]   BLOCK (exit -> @35F4)
[338F]     AWAIT presentation
[3390]     GUARD active_actor == Ulikan.talk (related 40)
[3395]     GUARD ti == 1
[339C]     ENDIF
[339D]     IF-BLOCK (exit -> @33C2)
[33A0]       OP_CB CB F5 19 0C CA 07
[33A6]       ENDIF
[33A7]       SAY "MERRY CHRISTMAS ..."
[33B5]       SETCHAR slot 2 = "christmas"
    END
[33C2]     IF-BLOCK (exit -> @33E4)
[33C5]       OP_CB CB F5 01 01 CA 07
[33CB]       ENDIF
[33CC]       SAY "HAPPY NEW YEAR ..."
[33DC]       SETCHAR slot 2 = "year"
    END
[33E4]     SAY "Network ..."
[33F0]     SAY "modem activated"
[33FC]     SAY "Rate 12,000 baud ..."
[340C]     SAY "Connect"
[3416]     SAY ".5"
[3420]     SAY ".4"
[342A]     SAY ".3"
[3434]     SAY ".2"
[343E]     SAY ".1"
[3448]     SAY ".0"
[3452]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[3462]     SAY "Pirate message : "CYBERION JUNIOR" calling. You're playing Commander Blood ... You got tips or codes ?"
[348C]     SAY "Let's swap codes ..."
[349C]     SAY "You found the robot Morning-Oil ... Teleport him ..."
[34B6]     SAY "Buy the batteries at VENUSIA , the space supramarket ..."
[34D2]     SAY "This is costing me too much ..."
[34E8]     IF-BLOCK (exit -> @351F)
[34EB]       OP_CA CA F1 C1 08 00
[34F0]       OP_CA CA F2 C1 02 00
[34F5]       ENDIF
[34F6]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[351C]       GOTO @35D6
    END
[351F]     IF-BLOCK (exit -> @3542)
[3522]       OP_CA CA F1 C1 0C 00
[3527]       OP_CA CA F2 C1 08 00
[352C]       ENDIF
[352D]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[353F]       GOTO @35D6
    END
[3542]     IF-BLOCK (exit -> @3575)
[3545]       OP_CA CA F2 C1 15 00
[354A]       OP_CA CA F1 C1 02 00
[354F]       ENDIF
[3550]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[3572]       GOTO @35D6
    END
[3575]     IF-BLOCK (exit -> @35A0)
[3578]       OP_CA CA F2 C1 12 00
[357D]       OP_CA CA F1 C1 15 00
[3582]       ENDIF
[3583]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[359D]       GOTO @35D6
    END
[35A0]     IF-BLOCK (exit -> @35D6)
[35A3]       OP_CA CA F2 C1 0C 00
[35A8]       OP_CA CA F1 C1 12 00
[35AD]       ENDIF
[35AE]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[35D6]     SAY "stop"  '[skip 4]
[35E0]     rec_00DC &= !0x2
[35E6]     POKE [0x3376] = 0
[35EA]     trak1 = 1
[35F1]     END PRESENTATION Ulikan.talk
  END
[35F4]   BLOCK (exit -> @360A)
[35F8]     GUARD ti == 2
[35FF]     ENDIF
[3600]     rec_00DC |= 0x2
[3605]     OP_C3 C3 14 01 28 00
  END
[360A]   BLOCK (exit -> @37D7)
[360E]     AWAIT presentation
[360F]     GUARD active_actor == Ulikan.talk (related 40)
[3614]     GUARD ti == 2
[361B]     ENDIF
[361C]     SAY "Network..."
[3626]     SAY "modem activated"
[3632]     SAY "Rate 24,000 baud ..."
[3642]     SAY "Connect"
[364C]     SAY ".3"
[3656]     SAY ".2"
[3660]     SAY ".1"
[366A]     SAY ".0"
[3674]     SAY "CONNECT COMMANDER BLOOD GAME"
[3684]     SAY "PIRATE MESSAGE FROM "CYBERION JUNIOR" TO OTHER GUY ..."
[369E]     SAY "I FOUND YOKO ON RONDO . ALSO DOCTOR OTTO VON SMILE ..."
[36BE]     SAY "THE GLUXX FAMILY AND SLIM GELATI..."
[36D2]     IF-BLOCK (exit -> @3709)
[36D5]       OP_CA CA F1 C1 08 00
[36DA]       OP_CA CA F2 C1 02 00
[36DF]       ENDIF
[36E0]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[3706]       GOTO @37C0
    END
[3709]     IF-BLOCK (exit -> @372C)
[370C]       OP_CA CA F1 C1 0C 00
[3711]       OP_CA CA F2 C1 08 00
[3716]       ENDIF
[3717]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[3729]       GOTO @37C0
    END
[372C]     IF-BLOCK (exit -> @375F)
[372F]       OP_CA CA F2 C1 15 00
[3734]       OP_CA CA F1 C1 02 00
[3739]       ENDIF
[373A]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[375C]       GOTO @37C0
    END
[375F]     IF-BLOCK (exit -> @378A)
[3762]       OP_CA CA F2 C1 12 00
[3767]       OP_CA CA F1 C1 15 00
[376C]       ENDIF
[376D]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[3787]       GOTO @37C0
    END
[378A]     IF-BLOCK (exit -> @37C0)
[378D]       OP_CA CA F2 C1 0C 00
[3792]       OP_CA CA F1 C1 12 00
[3797]       ENDIF
[3798]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[37C0]     SAY "stop"  '[skip 3]
[37CA]     rec_00DC &= !0x2
[37D0]     POKE [0x35F5] = 0
[37D4]     END PRESENTATION Ulikan.talk
  END
[37D7]   BLOCK (exit -> @37ED)
[37DB]     GUARD ti == 3
[37E2]     ENDIF
[37E3]     rec_00DC |= 0x2
[37E8]     OP_C3 C3 14 01 28 00
  END
[37ED]   BLOCK (exit -> @3974)
[37F1]     AWAIT presentation
[37F2]     GUARD active_actor == Ulikan.talk (related 40)
[37F7]     GUARD ti == 3
[37FE]     ENDIF
[37FF]     SAY "Network..."
[3809]     SAY "modem activated"
[3815]     SAY "rate 64,000 baud..."
[3823]     SAY "Connect"
[382D]     SAY ".2"
[3837]     SAY ".1"
[3841]     SAY ".0"
[384B]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[385B]     SAY "I FOUND THE LENS FOR MAXXON..."
[386F]     IF-BLOCK (exit -> @38A6)
[3872]       OP_CA CA F1 C1 08 00
[3877]       OP_CA CA F2 C1 02 00
[387C]       ENDIF
[387D]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[38A3]       GOTO @395D
    END
[38A6]     IF-BLOCK (exit -> @38C9)
[38A9]       OP_CA CA F1 C1 0C 00
[38AE]       OP_CA CA F2 C1 08 00
[38B3]       ENDIF
[38B4]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[38C6]       GOTO @395D
    END
[38C9]     IF-BLOCK (exit -> @38FC)
[38CC]       OP_CA CA F2 C1 15 00
[38D1]       OP_CA CA F1 C1 02 00
[38D6]       ENDIF
[38D7]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[38F9]       GOTO @395D
    END
[38FC]     IF-BLOCK (exit -> @3927)
[38FF]       OP_CA CA F2 C1 12 00
[3904]       OP_CA CA F1 C1 15 00
[3909]       ENDIF
[390A]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[3924]       GOTO @395D
    END
[3927]     IF-BLOCK (exit -> @395D)
[392A]       OP_CA CA F2 C1 0C 00
[392F]       OP_CA CA F1 C1 12 00
[3934]       ENDIF
[3935]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[395D]     SAY "stop"  '[skip 3]
[3967]     rec_00DC &= !0x2
[396D]     POKE [0x37D8] = 0
[3971]     END PRESENTATION Ulikan.talk
  END
[3974]   BLOCK (exit -> @398A)
[3978]     GUARD ti == 4
[397F]     ENDIF
[3980]     rec_00DC |= 0x2
[3985]     OP_C3 C3 14 01 28 00
  END
[398A]   BLOCK (exit -> @3B45)
[398E]     AWAIT presentation
[398F]     GUARD active_actor == Ulikan.talk (related 40)
[3994]     GUARD ti == 4
[399B]     ENDIF
[399C]     SAY "Network..."
[39A6]     SAY "modem activated"
[39B2]     SAY "rate 128,000 baud..."
[39C0]     SAY "Connect"
[39CA]     SAY ".5"
[39D4]     SAY ".4"
[39DE]     SAY ".3"
[39E8]     SAY ".2"
[39F2]     SAY ".1"
[39FC]     SAY ".0"
[3A06]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[3A16]     SAY "I TELEPORTED BRONKO ... HE'S BECOME THE HEAD CHEF ... THE FOOD'S BETTER , I THINK ..."
[3A40]     IF-BLOCK (exit -> @3A77)
[3A43]       OP_CA CA F1 C1 08 00
[3A48]       OP_CA CA F2 C1 02 00
[3A4D]       ENDIF
[3A4E]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[3A74]       GOTO @3B2E
    END
[3A77]     IF-BLOCK (exit -> @3A9A)
[3A7A]       OP_CA CA F1 C1 0C 00
[3A7F]       OP_CA CA F2 C1 08 00
[3A84]       ENDIF
[3A85]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[3A97]       GOTO @3B2E
    END
[3A9A]     IF-BLOCK (exit -> @3ACD)
[3A9D]       OP_CA CA F2 C1 15 00
[3AA2]       OP_CA CA F1 C1 02 00
[3AA7]       ENDIF
[3AA8]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[3ACA]       GOTO @3B2E
    END
[3ACD]     IF-BLOCK (exit -> @3AF8)
[3AD0]       OP_CA CA F2 C1 12 00
[3AD5]       OP_CA CA F1 C1 15 00
[3ADA]       ENDIF
[3ADB]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[3AF5]       GOTO @3B2E
    END
[3AF8]     IF-BLOCK (exit -> @3B2E)
[3AFB]       OP_CA CA F2 C1 0C 00
[3B00]       OP_CA CA F1 C1 12 00
[3B05]       ENDIF
[3B06]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[3B2E]     SAY "stop"  '[skip 3]
[3B38]     rec_00DC &= !0x2
[3B3E]     POKE [0x3975] = 0
[3B42]     END PRESENTATION Ulikan.talk
  END
[3B45]   BLOCK (exit -> @3B5B)
[3B49]     GUARD ti == 5
[3B50]     ENDIF
[3B51]     rec_00DC |= 0x2
[3B56]     OP_C3 C3 14 01 28 00
  END
[3B5B]   BLOCK (exit -> @3CDE)
[3B5F]     AWAIT presentation
[3B60]     GUARD active_actor == Ulikan.talk (related 40)
[3B65]     GUARD ti == 5
[3B6C]     ENDIF
[3B6D]     SAY "Network ..."
[3B79]     SAY "modem activated"
[3B85]     SAY "rate 240,000 baud..."
[3B93]     SAY ".2"
[3B9D]     SAY ".1"
[3BA7]     SAY ".0"
[3BB1]     SAY "CONNECTION... COMMANDER BLOOD GAME"
[3BC1]     SAY "HAVE YOU SEEN HOM ... WOW ! ..."
[3BD9]     IF-BLOCK (exit -> @3C10)
[3BDC]       OP_CA CA F1 C1 08 00
[3BE1]       OP_CA CA F2 C1 02 00
[3BE6]       ENDIF
[3BE7]       SAY "YOU SURE GET UP EARLY TO PLAY ... I GOTTA GO ... SEE YA !"  '[voice 17]
[3C0D]       GOTO @3CC7
    END
[3C10]     IF-BLOCK (exit -> @3C33)
[3C13]       OP_CA CA F1 C1 0C 00
[3C18]       OP_CA CA F2 C1 08 00
[3C1D]       ENDIF
[3C1E]       SAY "BEAUTIFUL MORNING ... SEE YA"  '[voice 16]
[3C30]       GOTO @3CC7
    END
[3C33]     IF-BLOCK (exit -> @3C66)
[3C36]       OP_CA CA F2 C1 15 00
[3C3B]       OP_CA CA F1 C1 02 00
[3C40]       ENDIF
[3C41]       SAY "NITE NITE ... DON'T STAY UP TOO LATE . TOMORROW ALWAYS COMES ..."  '[voice 15]
[3C63]       GOTO @3CC7
    END
[3C66]     IF-BLOCK (exit -> @3C91)
[3C69]       OP_CA CA F2 C1 12 00
[3C6E]       OP_CA CA F1 C1 15 00
[3C73]       ENDIF
[3C74]       SAY "HAVE FUN ... MY MOM SAYS SUPPER'S READY ..."  '[voice 14]
[3C8E]       GOTO @3CC7
    END
[3C91]     IF-BLOCK (exit -> @3CC7)
[3C94]       OP_CA CA F2 C1 0C 00
[3C99]       OP_CA CA F1 C1 12 00
[3C9E]       ENDIF
[3C9F]       SAY "THINK I'LL GO NOW ... AFTERNOON'S A GOOD TIME TO CATCH UP ON SOME ZZZZZ'S ..."  '[voice 13]
    END
[3CC7]     SAY "..."  '[skip 3]
[3CD1]     rec_00DC &= !0x2
[3CD7]     POKE [0x3B46] = 0
[3CDB]     END PRESENTATION Ulikan.talk
  END
[3CDE]   BLOCK (exit -> @3D5B)
[3CE2]     AWAIT gameflag_252A
[3CE3]     GUARD (rec_0B2C & 0x2) != 0
[3CE8]     GUARD G1 == 1
[3CEF]     GUARD active_actor == Izwalito.talk (related 40)
[3CF4]     ENDIF
[3CF5]     SAY "Izwalito happy see you , dear friend from sky and stars ..."  '[voice 2]
[3D15]     SAY "Me must go visit villages ..."  '[voice 2]
[3D29]     SAY "Me not can talk ..."  '[voice 3]
[3D3B]     SAY "Bye bye , dear friend from sky ..."  '[voice 3, skip 2]
[3D53]     rec_0572 = 3788
[3D58]     END PRESENTATION Izwalito.talk
  END
[3D5B]   BLOCK (exit -> @4146)
[3D5F]     AWAIT gameflag_252A
[3D60]     GUARD A1 == 0
[3D67]     GUARD rec_0F4E == 3380
[3D6C]     GUARD active_actor == Izwalito.talk (related 40)
[3D71]     ENDIF
[3D72]     SAY "Olga reporting for "instant translation" duty :"
[3D88]     SAY "Oooooh! You stranger ... You come from sky ... FEAR ... FEAR ..."  '[voice 2]
[3DAA]     SAY "YOU NOT KILL ME ... ME NICE . Me be IZWALITO ..."  '[voice 6]
[3DCA]     SAY "I don't think we can trust that creature , Commander . Better watch it carefully ..."
[3DF2]     SAY "It's a species of common space rat ..."
[3E0A]     SAY "Izwalito be very hungry ... Izwalito want eat ..."  '[voice 2]
[3E24]     SAY "It's just what I figured , Commander . He wants our food ..."
[3E46]     SAY "You say your name to Izwalito , stranger ? word_65535 hyper_man predatorus commander_blood Mousy jules_verne Exterminator tarzoom"  '[voice 2]
[3E72]     IF-BLOCK (exit -> @3E91)
[3E75]       GUARD concept == "commander_blood"
[3E78]       ENDIF
[3E79]       SAY "Me greet you Commander."  '[voice 2, skip 1]
[3E89]       blo = 1
[3E90]       CLEAR concept_alt
    END
[3E91]     IF-BLOCK (exit -> @3EB0)
[3E94]       GUARD concept == "hyper_man"
[3E97]       ENDIF
[3E98]       SAY "Me greet you Hyper"  '[voice 0, skip 1]
[3EA8]       sup = 1
[3EAF]       CLEAR concept_alt
    END
[3EB0]     IF-BLOCK (exit -> @3ECF)
[3EB3]       GUARD concept == "predatorus"
[3EB6]       ENDIF
[3EB7]       SAY "Me greet you Predatorus"  '[voice 0, skip 1]
[3EC7]       pre = 1
[3ECE]       CLEAR concept_alt
    END
[3ECF]     IF-BLOCK (exit -> @3EF0)
[3ED2]       GUARD concept == "Mousy"
[3ED5]       ENDIF
[3ED6]       SAY "Me greet you friend Mousy"  '[voice 0, skip 1]
[3EE8]       mic = 1
[3EEF]       CLEAR concept_alt
    END
[3EF0]     IF-BLOCK (exit -> @3F11)
[3EF3]       GUARD concept == "jules_verne"
[3EF6]       ENDIF
[3EF7]       SAY "Me greet you friend Jules"  '[voice 0, skip 1]
[3F09]       jul = 1
[3F10]       CLEAR concept_alt
    END
[3F11]     IF-BLOCK (exit -> @3F32)
[3F14]       GUARD concept == "Exterminator"
[3F17]       ENDIF
[3F18]       SAY "Me greet you friend Exterminator"  '[voice 0, skip 1]
[3F2A]       ter = 1
[3F31]       CLEAR concept_alt
    END
[3F32]     IF-BLOCK (exit -> @3F53)
[3F35]       GUARD concept == "tarzoom"
[3F38]       ENDIF
[3F39]       SAY "Me greet you friend Tarzoom"  '[voice 0, skip 1]
[3F4B]       tar = 1
[3F52]       CLEAR concept_alt
    END
[3F53]     SAY "HONK REPORTS IN : ... ... ..."
[3F69]     SAY "This Izwalito joker doesn't trust you , Commander ..."
[3F83]     SAY "Try to humor him . We'll soon find out the truth behind that innocent mask ... Heh ! Heh !"  '[skip 1]
[3FB3]     rec_05A0 = 1
[3FB8]     IF-BLOCK (exit -> @405E)
[3FBB]       GUARD secret == 1
[3FC2]       GUARD (rec_0B2C & 0x2) == 0
[3FC8]       ENDIF
[3FC9]       SAY "You be very curious , friend ..."  '[voice 2]
[3FDF]       SAY "You can keep secret ? word_65535 yes no"  '[voice 3]
[3FF9]       IF-BLOCK (exit -> @4020)
[3FFC]         GUARD NOT concept == "yes"
[4000]         ENDIF
[4001]         SAY "Me tell big secret ..."  '[voice 2, skip 3]
[4013]         rec_05A0 = 4785
[4018]         secret = 0
[401F]         CLEAR concept_alt
      END
[4020]       IF-BLOCK (exit -> @405E)
[4023]         GUARD concept == "no"
[4026]         ENDIF
[4027]         SAY "Commander !!! You're being manipulated by a rat ... You should've answered YES ..."
[404B]         SAY "You like make joke ..."  '[voice 4, skip 1]
[405D]         CLEAR concept_alt
      END
    END
[405E]     SAY "STOP"  '[skip 1]
[4068]     state[2] = 80
[406C]     IF-BLOCK (exit -> @407A)
[406F]       GUARD state[2] == 0
[4071]       ENDIF
[4072]       state[2] = 65535
[4076]       POKE [0x4147] = 1
    END
[407A]     SAY "You bye bye ? word_65535 bye_bye"  '[skip 1]
[408E]     adieu = 1
[4095]     IF-BLOCK (exit -> @40D8)
[4098]       GUARD (rec_0C04 & 0x2) != 0
[409D]       ENDIF
[409E]       SAY "HONK REPORTING AT THIS TIME:"
[40B0]       SAY "COMMANDER , YOU'RE NOT GONNA BELIEVE THIS ... THE RAT GAVE US A PLANETARY POSITION ..."
    END
[40D8]     IF-BLOCK (exit -> @4130)
[40DB]       GUARD (rec_0B2C & 0x2) != 0
[40E0]       ENDIF
[40E1]       SAY "COMPUTER ENTITY HONK :"
[40F1]       SAY "COMMANDER , Ratface knows a lot more than he's telling ..."
[410F]       SAY "You should get him to spill the beans ..."  '[skip 1]
[4129]       trak2c = 1
    END
[4130]     IF-BLOCK (exit -> @4146)
[4133]       GUARD adieu == 1
[413A]       ENDIF
[413B]       POKE [0x4147] = 1
[413F]       adieu = -1.value
    END
  END
[4146]   GOTO @440A
[414A]   AWAIT gameflag_252A
[414B]   ENDIF
[414C]   IF-BLOCK (exit -> @41E6)
[414F]     GUARD (rec_0C04 & 0x2) == 0
[4155]     ENDIF
[4156]     SAY "If you have garbage, me give you coodinates of intergalactic dump, friend ..."  '[voice 3]
[4178]     SAY "Planet Magnus be in Magnalus x342 y543 system ... There be intergalactic garbage dump"  '[voice 5, skip 1]
[419C]     rec_0C04 |= 0x2
[41A1]     SAY "Commander , he gave us a planetary position ... Don't you just love these ratfaced folks ..."
[41CB]     SAY "You promise not pollute, friend ..."  '[voice 6, skip 1]
[41DF]     trak2b = 1
  END
[41E6]   SAY "Me must go , friend ... Me have big date with female ..."  '[voice 2]
[4208]   SAY "You come back in moment ! Me wait for you , nice friend ..."  '[voice 3]
[422C]   SAY "You leave , quick ..."  '[voice 5]
[423E]   IF-BLOCK (exit -> @4281)
[4241]     GUARD mic == 1
[4248]     ENDIF
[4249]     SAY "See you soon , friend Mousy ..."  '[voice 5]
[425F]     SAY "Me not believe you be Mousy . You not have giant ears ..."  '[voice 2]
  END
[4281]   IF-BLOCK (exit -> @42C2)
[4284]     GUARD ter == 1
[428B]     ENDIF
[428C]     SAY "See you soon , friend Exterminator ..."  '[voice 5]
[42A2]     SAY "Me not believe you be Exterminator . Where be truck ? ..."  '[voice 2]
  END
[42C2]   IF-BLOCK (exit -> @4307)
[42C5]     GUARD jul == 1
[42CC]     ENDIF
[42CD]     SAY "See you soon , friend Jules ..."  '[voice 5]
[42E3]     SAY "Me not believe you be Jules Verne . You not be so old ..."  '[voice 2]
  END
[4307]   IF-BLOCK (exit -> @4328)
[430A]     GUARD blo == 1
[4311]     ENDIF
[4312]     SAY "See you soon , friend Commander ."  '[voice 5]
  END
[4328]   IF-BLOCK (exit -> @4369)
[432B]     GUARD pre == 1
[4332]     ENDIF
[4333]     SAY "See you soon , friend Predatorus ..."  '[voice 5]
[4349]     SAY "Me not believe you be Predatorus . You easy to see ..."  '[voice 5]
  END
[4369]   IF-BLOCK (exit -> @43A8)
[436C]     GUARD tar == 1
[4373]     ENDIF
[4374]     SAY "See you soon , friend Tarzoom ..."  '[voice 5]
[438A]     SAY "Me not believe you be Tarzoom ... You not yodel ..."  '[voice 2]
  END
[43A8]   IF-BLOCK (exit -> @43EB)
[43AB]     GUARD sup == 1
[43B2]     ENDIF
[43B3]     SAY "See you soon , friend Hyper ..."  '[voice 5]
[43C9]     SAY "Me not believe you be Hyper Man ... Where be cape ? ..."  '[voice 2]
  END
[43EB]   SAY "stop"  '[skip 4]
[43F5]   A1 = 1
[43FC]   trak2 = 1
[4403]   POKE [0x4147] = 0
[4407]   END PRESENTATION Izwalito.talk
[440A]   BLOCK (exit -> @480F)
[440E]     AWAIT gameflag_252A
[440F]     GUARD rec_0F4E == 3380
[4414]     GUARD A1 == 1
[441B]     GUARD active_actor == Izwalito.talk (related 40)
[4420]     ENDIF
[4421]     SAY "Olga on station for "simul-tran" services :"
[4437]     SAY "Yipee ! Me happy ... You come back ..."  '[voice 5]
[4451]     SAY "Me like you ..."  '[voice 6]
[4461]     SAY "Izwal people be very hungry . We starve . You help us ..."  '[voice 7]
[4483]     SAY "You go planet MOSKITO , buy frozen murffalo meat ..."  '[voice 1]
[449F]     SAY "Me give you one CRED to buy meat ..."  '[voice 5]
[44B9]     SAY "TELEPORT CRED TO ARK word_65535 teleport refuse"
[44D1]     IF-BLOCK (exit -> @44F0)
[44D4]       GUARD concept == "teleport"
[44D7]       ENDIF
[44D8]       SAY "CRED TELEPORTED TO ARK"  '[skip 2]
[44E8]       OP_CD CD 94 05 04 10 28 00
[44EF]       CLEAR concept_alt
    END
[44F0]     IF-BLOCK (exit -> @453D)
[44F3]       GUARD concept == "refuse"
[44F6]       ENDIF
[44F7]       SAY "Commander , you should have accepted ..."
[450D]       SAY "Me sad . Me not like you ... Bye bye ..."  '[voice 5]
[452B]       SAY "Bye bye ..."  '[voice 2, skip 2]
[4539]       CLEAR concept_alt
[453A]       END PRESENTATION Izwalito.talk
    END
[453D]     SAY "Moskito be 124325 degrees in galactic cluster B12"  '[voice 2, skip 1]
[4555]     rec_0EEC |= 0x2
[455A]     SAY "COMMANDER , YOU'RE NOT GONNA BELIEVE THIS ... THE RAT GAVE US A PLANETARY POSITION ..."
[4582]     SAY "You take care of CRED , friend ..."  '[voice 1]
[459A]     SAY "Me look forward see you back , friend from sky and stars ..."  '[voice 7]
[45BC]     SAY "Izwal people trust you ..."  '[voice 6]
[45CE]     SAY "REPORT FROM HONK , ONBOARD BIOCONSCIOUSNESS :"
[45E4]     SAY "Have you noticed that rat has a trunk , Commander ? Believe me , that's bad ..."
[460E]     SAY "Trunks generally mean trouble ... And rats with trunks are the worst ..."
[4630]     SAY "STOP"
[463A]     IF-BLOCK (exit -> @467B)
[463D]       GUARD mic == 1
[4644]       ENDIF
[4645]       SAY "See you soon , friend Mousy ..."  '[voice 5]
[465B]       SAY "Me not believe you be Mousy . You not have tail ..."  '[voice 2]
    END
[467B]     IF-BLOCK (exit -> @46BC)
[467E]       GUARD ter == 1
[4685]       ENDIF
[4686]       SAY "See you soon , friend Exterminator ..."  '[voice 5]
[469C]       SAY "Me not believe you be Exterminator . You not have shades ..."  '[voice 2]
    END
[46BC]     IF-BLOCK (exit -> @4701)
[46BF]       GUARD jul == 1
[46C6]       ENDIF
[46C7]       SAY "See you soon , friend Jules ..."  '[voice 5]
[46DD]       SAY "Me not believe you be Jules Verne . You not have white beard ..."  '[voice 2]
    END
[4701]     IF-BLOCK (exit -> @4720)
[4704]       GUARD blo == 1
[470B]       ENDIF
[470C]       SAY "See you soon , friend Commander"  '[voice 5]
    END
[4720]     IF-BLOCK (exit -> @4761)
[4723]       GUARD pre == 1
[472A]       ENDIF
[472B]       SAY "See you soon , friend Predatorus ..."  '[voice 5]
[4741]       SAY "Me not believe you be Predatorus . His teeth be pointy ..."  '[voice 5]
    END
[4761]     IF-BLOCK (exit -> @47A0)
[4764]       GUARD tar == 1
[476B]       ENDIF
[476C]       SAY "See you soon , friend Tarzoom ..."  '[voice 5]
[4782]       SAY "Me not believe you be Tarzoom ... You speak good ..."  '[voice 2]
    END
[47A0]     IF-BLOCK (exit -> @47ED)
[47A3]       GUARD sup == 1
[47AA]       ENDIF
[47AB]       SAY "See you soon , friend Hyper ..."  '[voice 5]
[47C1]       SAY "Me not believe you be Hyper Man ... You not have cute lock of hair on forehead ..."  '[voice 2]
    END
[47ED]     SAY "stop"  '[skip 4]
[47F7]     A1 = 2
[47FE]     A11 = 1
[4805]     trak3 = 1
[480C]     END PRESENTATION Izwalito.talk
  END
[480F]   BLOCK (exit -> @4837)
[4813]     AWAIT gameflag_252A
[4814]     GUARD rec_1078 == 40
[4819]     GUARD NOT rec_1060 == 1370
[481F]     GUARD NOT rec_1060 == 40
[4825]     GUARD NOT rec_1018 == 1370
[482B]     ENDIF
[482C]     zob1 = 1
[4833]     POKE [0x4810] = 0
  END
[4837]   BLOCK (exit -> @49EC)
[483B]     AWAIT gameflag_252A
[483C]     GUARD zob1 == 1
[4843]     ENDIF
[4844]     SAY "Olga now ready for "sim tran" mission :"
[485C]     SAY "Me greet you , friend ..."  '[voice 2]
[4870]     SAY "Izwal people of Corpo wait you . You did buy murffalo on planet MOSKITO with CRED ?"  '[voice 3]
[489A]     SAY "Commander , give Ratso the murffalo we got at the VENUSIA SUPRAMARKET"
[48BA]     SAY "HE WON'T KNOW THE DIFFERENCE ... Ha! Ha! Ha!"
[48D4]     SAY "TELEPORT MURFFALO TO IZWALITO word_65535 teleport"
[48EA]     IF-BLOCK (exit -> @4909)
[48ED]       GUARD concept == "teleport"
[48F0]       ENDIF
[48F1]       SAY "MURFFALO TELEPORTED TO IZWALITO"  '[skip 1]
[4901]       OP_CD CD 30 00 64 10 5A 05
[4908]       CLEAR concept_alt
    END
[4909]     SAY "This not be murffalo from MOSKITO, friend . You make big mistake ..."  '[voice 5]
[492B]     SAY "This murffalo be too expensive and very small . Me ask big juicy murffalo from planet Moskito ..."  '[voice 4]
[4957]     SAY "CRY CRY ...."  '[voice 5]
[4965]     SAY "YOU SPEND CRED ... CRY CRY ..."  '[voice 4]
[497B]     SAY "COMMANDER, it's just what I thought ... This poor rat is smarter than you thought . I suggest you make up for your error ..."
[49B5]     SAY "BYE BYE ... IZWALS UNHAPPY NOW .... CRY DESPAIR ...."  '[voice 3]
[49D1]     SAY "stop"  '[skip 3]
[49DB]     trak4 = 1
[49E2]     zob1 = 0
[49E9]     END PRESENTATION Izwalito.talk
  END
[49EC]   BLOCK (exit -> @4CEA)
[49F0]     AWAIT gameflag_252A
[49F1]     GUARD rec_0F4E == 3380
[49F6]     GUARD A11 == 1
[49FD]     GUARD A1 == 2
[4A04]     GUARD active_actor == Izwalito.talk (related 40)
[4A09]     ENDIF
[4A0A]     SAY "You be back ..."  '[voice 5]
[4A1A]     SAY "You did buy murffalo meat ?"  '[voice 7]
[4A2E]     SAY "We starve . You help us ... Me give you CRED ..."  '[voice 5]
[4A4E]     SAY "You go planet MOSKITO, buy frozen murffalo meat ..."  '[voice 1]
[4A68]     IF-BLOCK (exit -> @4AC7)
[4A6B]       GUARD rec_1018 == 40
[4A70]       ENDIF
[4A71]       SAY "ONBOARD COMPUTING BRAIN HONK REPORTS :"
[4A85]       SAY "Commander, Ratso doesn't seem too happy ..."
[4A9B]       SAY "If he's really that hungry , maybe he can eat his CRED .."
[4ABD]       SAY "..."
    END
[4AC7]     IF-BLOCK (exit -> @4B17)
[4ACA]       GUARD NOT rec_1018 == 40
[4AD0]       ENDIF
[4AD1]       SAY "ONBOARD COMPUTING FACILITY HONK REPORTS :"
[4AE5]       SAY "Commander, something tells me Cap'n Bob won't be happy about the big mistake you've made ..."
[4B0D]       SAY "..."
    END
[4B17]     SAY "Izwal people look forward see you back ..."  '[voice 6]
[4B2F]     IF-BLOCK (exit -> @4B6E)
[4B32]       GUARD mic == 1
[4B39]       ENDIF
[4B3A]       SAY "See you soon , friend Mousy ..."  '[voice 5]
[4B50]       SAY "Me not believe you be Mousy . You not squeak ..."  '[voice 2]
    END
[4B6E]     IF-BLOCK (exit -> @4BAD)
[4B71]       GUARD ter == 1
[4B78]       ENDIF
[4B79]       SAY "See you soon , friend Exterminator ..."  '[voice 5]
[4B8F]       SAY "Me not believe you be Exterminator . You not terminate ..."  '[voice 2]
    END
[4BAD]     IF-BLOCK (exit -> @4BEE)
[4BB0]       GUARD jul == 1
[4BB7]       ENDIF
[4BB8]       SAY "See you soon , friend Jules ..."  '[voice 5]
[4BCE]       SAY "Me not believe you be Jules Vernes. You not smell good ..."  '[voice 2]
    END
[4BEE]     IF-BLOCK (exit -> @4C0D)
[4BF1]       GUARD blo == 1
[4BF8]       ENDIF
[4BF9]       SAY "See you soon , friend Commander"  '[voice 5]
    END
[4C0D]     IF-BLOCK (exit -> @4C52)
[4C10]       GUARD pre == 1
[4C17]       ENDIF
[4C18]       SAY "See you soon , friend Predatorus ..."  '[voice 5]
[4C2E]       SAY "Me not believe you be Predatorus . Him chop Izwals in meaty chunks ..."  '[voice 5]
    END
[4C52]     IF-BLOCK (exit -> @4C91)
[4C55]       GUARD tar == 1
[4C5C]       ENDIF
[4C5D]       SAY "See you soon , friend Tarzoom ..."  '[voice 5]
[4C73]       SAY "Me not believe you be Tarzoom ... Tarzoom smell worse ..."  '[voice 2]
    END
[4C91]     IF-BLOCK (exit -> @4CD6)
[4C94]       GUARD sup == 1
[4C9B]       ENDIF
[4C9C]       SAY "See you soon , friend Hyper ..."  '[voice 5]
[4CB2]       SAY "Me not believe you be Hyper Man ... Alter ego not wear glasses ..."  '[voice 2]
    END
[4CD6]     SAY "stop"  '[skip 2]
[4CE0]     A11 = 2
[4CE7]     END PRESENTATION Izwalito.talk
  END
[4CEA]   BLOCK (exit -> @4E2E)
[4CEE]     AWAIT gameflag_252A
[4CEF]     GUARD rec_0F4E == 3380
[4CF4]     GUARD A11 == 2
[4CFB]     GUARD A1 == 2
[4D02]     GUARD active_actor == Izwalito.talk (related 40)
[4D07]     ENDIF
[4D08]     SAY "You again ..."  '[voice 5]
[4D16]     SAY "You not buy frozen murffalo meat ?"  '[voice 7]
[4D2C]     SAY "We starve . You help us ... You give back CRED !!!"  '[voice 5]
[4D4C]     SAY "You go planet MOSKITO , buy frozen murffalo meat ..."  '[voice 1]
[4D68]     IF-BLOCK (exit -> @4DDC)
[4D6B]       GUARD NOT rec_1018 == 40
[4D71]       ENDIF
[4D72]       SAY "HONK HERE :"
[4D80]       SAY "Commander , didn't you spend that poor little rat's CRED ??"  '[skip 1]
[4D9E]       vol = 1
[4DA5]       SAY "That's called THEFT , COMMANDER . You'll have to get Ratso what he wants ..."  '[skip 1]
[4DCB]       PP1 = 1
[4DD2]       SAY "STOP"
    END
[4DDC]     SAY "Me look forward see you , friend from sky and stars ..."  '[voice 7]
[4DFC]     SAY "Izwal people starve . You bring murffalo from planet MOSKITO ..."  '[voice 6]
[4E1A]     SAY "stop"  '[skip 2]
[4E24]     A11 = 3
[4E2B]     END PRESENTATION Izwalito.talk
  END
[4E2E]   BLOCK (exit -> @4F0E)
[4E32]     AWAIT gameflag_252A
[4E33]     GUARD rec_0F4E == 3380
[4E38]     GUARD A11 == 3
[4E3F]     GUARD A1 == 2
[4E46]     GUARD active_actor == Izwalito.talk (related 40)
[4E4B]     ENDIF
[4E4C]     SAY "You again ..."  '[voice 5]
[4E5A]     SAY "You not buy frozen murffalo meat from planet MOSKITO ?"  '[voice 7]
[4E76]     SAY "We starve . You help us ... You give back CRED !!!"  '[voice 5]
[4E96]     IF-BLOCK (exit -> @4EE5)
[4E99]       GUARD NOT rec_1018 == 40
[4E9F]       ENDIF
[4EA0]       SAY "Commander , Cap'n Bob won't like it when he finds out you stole that little guy's CRED ..."
[4ECC]       SAY "Where to find CREDs ..."  '[skip 1]
[4EDE]       PP1 = 1
    END
[4EE5]     SAY "CRY CRY ... Me not want see you again ..."
[4F01]     SAY "stop"  '[skip 1]
[4F0B]     END PRESENTATION Izwalito.talk
  END
[4F0E]   BLOCK (exit -> @4F3E)
[4F12]     AWAIT presentation
[4F13]     GUARD NOT rec_1018 == 1370
[4F19]     GUARD NOT rec_1060 == 1370
[4F1F]     GUARD A11 == 3
[4F26]     GUARD rec_0F4E == 2336
[4F2B]     ENDIF
[4F2C]     OP_C3 C3 94 05 28 00
[4F31]     rec_055C |= 0x2
[4F36]     POKE [0x4F3F] = 1
[4F3A]     POKE [0x4F0F] = 0
  END
[4F3E]   GOTO @500C
[4F42]   AWAIT presentation
[4F43]   ENDIF
[4F44]   SAY "RADIO MESSAGE :"
[4F52]   SAY "Hello ... I BE BOSSANOVA , wife of Izwalito ..."
[4F6E]   SAY "ME NOT HAPPY . IZWALITO DID GIVE YOU SAVINGS ... YOU GIVE BACK CRED !!!"
[4F94]   SAY "Izwal people of Corpo die of hunger ... CRY ... CRY ..."
[4FB4]   SAY "You bring murffalo , or you in trouble with BOSSANOVA ..."
[4FD2]   SAY "IZWALITO CRY MUCH ..."
[4FE2]   SAY "CRUIKKK !!!"
[4FEE]   SAY "stop"  '[skip 4]
[4FF8]   trak5 = 1
[4FFF]   POKE [0x4F3F] = 0
[5003]   rec_055C &= !0x2
[5009]   END PRESENTATION Izwalito.talk
[500C]   BLOCK (exit -> @530A)
[5010]     AWAIT gameflag_252A
[5011]     GUARD rec_0F4E == 3380
[5016]     GUARD A1 == 3
[501D]     GUARD active_actor == Izwalito.talk (related 40)
[5022]     ENDIF
[5023]     IF-BLOCK (exit -> @50FE)
[5026]       GUARD rec_1060 == 40
[502B]       ENDIF
[502C]       SAY "Friend , you have murffalo meat ? We starve ..."  '[voice 0]
[5048]       SAY "YOU HAVE MURFFALO !!! !!!"  '[voice 3]
[505A]       SAY "You've made one little rat very happy , Commander . Why not teleport him the meat ..."
[5084]       SAY "TELEPORT MURFFALO TO IZWALITO word_65535 teleport refuse"
[509C]       IF-BLOCK (exit -> @50BB)
[509F]         GUARD concept == "teleport"
[50A2]         ENDIF
[50A3]         SAY "MURFFALO TELEPORTED TO IZWALITO"  '[skip 2]
[50B3]         OP_CD CD 30 00 4C 10 5A 05
[50BA]         CLEAR concept_alt
      END
[50BB]       IF-BLOCK (exit -> @50FE)
[50BE]         GUARD concept == "refuse"
[50C1]         ENDIF
[50C2]         SAY "Commander , you're not being reasonable ..."
[50D8]         SAY "Me unhappy ... Me angry ..."  '[voice 6]
[50EC]         SAY "Bye bye ..."  '[voice 5, skip 2]
[50FA]         CLEAR concept_alt
[50FB]         END PRESENTATION Izwalito.talk
      END
    END
[50FE]     IF-BLOCK (exit -> @528E)
[5101]       GUARD rec_1060 == 1370
[5106]       ENDIF
[5107]       SAY "Oh, me happy now . Izwal people say thanks . Me give you big major gifts ..."  '[voice 2]
[5131]       SAY "You can see TV with DECODER . Me give you DECODER for IZWAL CHANNEL ..."  '[voice 5]
[5157]       SAY "A decoder , Commander ! Now we can have some fun for a change ..."
[517D]       SAY "Thanks to DECODER , you can see TV, friend..."  '[voice 3, skip 2]
[5197]       SETCHAR slot 1 = "ppit"
[519F]       SETCHAR slot 5 = "present"
[51AA]       SAY "TELEPORT DECODER TO ARK word_65535 teleport"
[51C0]       IF-BLOCK (exit -> @51DF)
[51C3]         GUARD concept == "teleport"
[51C6]         ENDIF
[51C7]         SAY "DECODER TELEPORTED TO ARK"  '[skip 2]
[51D7]         OP_CD CD 94 05 34 10 28 00
[51DE]         CLEAR concept_alt
      END
[51DF]       SAY "You love decoder , friend ..."  '[voice 2]
[51F3]       SAY "To reward you , me give you one CRED , friend ..."  '[voice 2]
[5213]       SAY "A cred . Ratboy's handing over a free cred ! ... I knew it ... These guys are all millionaires ..."
[5245]       SAY "TELEPORT CRED TO ARK word_65535 teleport"
[525B]       IF-BLOCK (exit -> @528E)
[525E]         GUARD concept == "teleport"
[5261]         ENDIF
[5262]         SAY "CRED TELEPORTED TO ARK"  '[skip 2]
[5272]         OP_CD CD 94 05 04 10 28 00
[5279]         CLEAR concept_alt
[527A]         SAY "You love cred , friend ..."  '[voice 2]
      END
    END
[528E]     SAY "You come back when want , friend ... word_65535 bye_bye"  '[voice 2, skip 1]
[52AA]     adieu = 1
[52B1]     IF-BLOCK (exit -> @530A)
[52B4]       GUARD adieu == 1
[52BB]       ENDIF
[52BC]       SAY "Me go quick give murffalo to starving Izwals ..."
[52D6]       SAY "Bye bye , friend ..."
[52E8]       SAY "stop"  '[skip 4]
[52F2]       trak6 = 1
[52F9]       adieu = 0
[5300]       A1 = 4
[5307]       END PRESENTATION Izwalito.talk
    END
  END
[530A]   BLOCK (exit -> @54C9)
[530E]     AWAIT gameflag_252A
[530F]     GUARD rec_1060 == 1370
[5314]     GUARD (rec_0B2C & 0x2) == 0
[531A]     GUARD rec_0F4E == 3380
[531F]     GUARD active_actor == Izwalito.talk (related 40)
[5324]     ENDIF
[5325]     SAY "Friend from sky and stars back on Corpo ..."  '[voice 2, skip 1]
[533F]     secret = 0
[5346]     SAY "Me happy , friend ..."  '[voice 3]
[5358]     SAY "You want what ? ..."  '[voice 4, skip 1]
[536A]     rec_05A0 = 1
[536F]     IF-BLOCK (exit -> @5418)
[5372]       GUARD secret == 1
[5379]       ENDIF
[537A]       SAY "You be very curious , friend ..."  '[voice 2]
[5390]       SAY "You can keep secret ? word_65535 yes no"  '[voice 3]
[53AA]       IF-BLOCK (exit -> @53D2)
[53AD]         GUARD concept == "yes"
[53B0]         ENDIF
[53B1]         SAY "Me say you big secret ..."  '[voice 2, skip 3]
[53C5]         rec_05A0 = 4785
[53CA]         secret = 0
[53D1]         CLEAR concept_alt
      END
[53D2]       IF-BLOCK (exit -> @5418)
[53D5]         GUARD concept == "no"
[53D8]         ENDIF
[53D9]         SAY "Commander !!! How can you be so lacking in perspicacity ? You were supposed to say YES ..."
[5405]         SAY "You like make joke ..."  '[voice 4, skip 1]
[5417]         CLEAR concept_alt
      END
    END
[5418]     IF-BLOCK (exit -> @549E)
[541B]       GUARD (rec_0B2C & 0x2) == 0
[5421]       GUARD rec_0590 > 5
[5428]       ENDIF
[5429]       SAY "You go see rich Izwals on planet Rondo, friend ... You say them hello from me ..."  '[voice 4]
[5453]       SAY "Rondo be in Rondalus system , coordinates x345 y432 ..."  '[voice 5, skip 1]
[546F]       rec_0B2C |= 0x2
[5474]       SAY "Commander , Commander he just gave us a new planetary position would you believe , Commander ..."
    END
[549E]     SAY "Bye bye , come back ... You talk good . Me like talk .... word_65535 bye_bye"  '[voice 5, skip 1]
[54C6]     END PRESENTATION Izwalito.talk
  END
[54C9]   BLOCK (exit -> @560F)
[54CD]     AWAIT gameflag_252A
[54CE]     GUARD rec_1060 == 1370
[54D3]     GUARD G1 == 0
[54DA]     GUARD (rec_0D54 & 0x2) == 0
[54E0]     GUARD (rec_0B2C & 0x2) != 0
[54E5]     GUARD rec_0F4E == 3380
[54EA]     GUARD active_actor == Izwalito.talk (related 40)
[54EF]     ENDIF
[54F0]     SAY "Izwalito happy see you , friend from sky and stars ..."  '[voice 2, skip 1]
[550E]     POKE [0x500D] = 0
[5512]     SAY "We receive big visit from His Sacredness HOM ... Hom be in next village ..."  '[voice 2]
[5538]     SAY "Village be Hita..."  '[voice 3]
[5546]     SAY "His Sacredness wants to see you , friend ..."  '[voice 3]
[5560]     SAY "You agree go Hita? word_65535 YES NO"  '[voice 2]
[5578]     IF-BLOCK (exit -> @55CA)
[557B]       GUARD concept == "YES"
[557E]       ENDIF
[557F]       SAY "You go north through desert to Hita ..."  '[voice 6]
[5597]       SAY "See you soon , friend ..."  '[voice 2]
[55AB]       SAY "..."  '[skip 5]
[55B5]       POKE [0x5610] = 1
[55B9]       POKE [0x54CA] = 0
[55BD]       A1 = 5
[55C4]       OP_C1 C1 4E 12 52 0D
[55C9]       CLEAR concept_alt
    END
[55CA]     IF-BLOCK (exit -> @560F)
[55CD]       GUARD concept == "NO"
[55D0]       ENDIF
[55D1]       SAY "Maybe you should've agreed , Commander ..."
[55E7]       SAY "Bye bye , friend . Me must go ..."  '[voice 4]
[5601]       SAY "stop"  '[skip 2]
[560B]       CLEAR concept_alt
[560C]       END PRESENTATION Izwalito.talk
    END
  END
[560F]   GOTO @5763
[5613]   AWAIT gameflag_252A
[5614]   GUARD rec_01CA == 3410
[5619]   GUARD G1 == 0
[5620]   GUARD active_actor == Hom.talk (related 40)
[5625]   ENDIF
[5626]   POKE [0x54CA] = 0
[562A]   SAY "Welcome stranger . You be big traveller ..."  '[voice 2]
[5642]   SAY "WOWEE !! Commander, this guy's even uglier than ol' Buzzard Face Bob ... I gotta have a snapshot ... Ha ! Ha ! Ha !"
[567C]   SAY "Me HOM . Me big tube brain . Me live on distant planet ..."  '[voice 2]
[56A0]   SAY "Me invite you to my planet , stranger ..."  '[voice 4]
[56BA]   SAY "You did help Izwals . Very good ... You must join MEMBERS GUILD"  '[voice 9]
[56DC]   SAY "You go my planet . I go back in few days ..."  '[voice 10]
[56FC]   SAY "My planet be KORTEX ..."  '[voice 3]
[570E]   SAY "Planet KORTEX be on ZXDT4534 in constellation of KORTESIUS ... YOU GO , FRIEND ..."  '[voice 9, skip 1]
[5734]   rec_0AF0 |= 0x2
[5739]   SAY "Bye bye , good stranger ..."  '[skip 4]
[574D]   trak7 = 1
[5754]   G1 = 1
[575B]   rec_01CA = 3788
[5760]   END PRESENTATION Hom.talk
[5763]   BLOCK (exit -> @5922)
[5767]     AWAIT gameflag_252A
[5768]     GUARD rec_0F4E == 3818
[576D]     GUARD rec_0350 == 1
[5774]     GUARD F1 == 0
[577B]     GUARD active_actor == Bronko.talk (related 40)
[5780]     ENDIF
[5781]     SAY "MURFFALO FACTORY"  '[skip 1]
[578D]     LOADSTR "moskit20.hnm"
[579C]     SAY "Me not have time to talk , stranger . Me have much work . You come back later ..."
[57CA]     SAY "Me have fifty tons of murffalos to chop up ... You come back later ..."  '[voice 6]
[57F0]     SAY "Me not play ... Me work ... You understand ..."  '[voice 7]
[580C]     SAY "This guy's no fun , Commander... Let's forget him ..."
[5828]     SAY "Bye bye ... Bronko very busy , much lot work ... You come back ..."  '[voice 5]
[584E]     SAY "I could be misreading this it, but I don't think he wants to talk, Commander..."
[5874]     IF-BLOCK (exit -> @58B4)
[5877]       GUARD (rec_0922 & 0x2) != 0
[587C]       GUARD rec_1018 == 40
[5881]       ENDIF
[5882]       SAY "Maybe we should go spend our fortune at the VENUSIA supramarket, Commander. There's bound to be great stuff to buy ..."
    END
[58B4]     IF-BLOCK (exit -> @58F1)
[58B7]       GUARD (rec_0922 & 0x2) == 0
[58BD]       GUARD rec_1018 == 40
[58C2]       ENDIF
[58C3]       SAY "Let's keep the CRED, Commander. There's bound to be zillions of great things to do with a CRED ..."
    END
[58F1]     SAY "Bye bye... Bronko real busy. Much work ... You come back later ..."  '[voice 5]
[5913]     SAY "Bye bye"  '[skip 1]
[591F]     END PRESENTATION Bronko.talk
  END
[5922]   BLOCK (exit -> @5C83)
[5926]     AWAIT gameflag_252A
[5927]     GUARD rec_0F4E == 3818
[592C]     GUARD A1 == 2
[5933]     GUARD F1 == 0
[593A]     GUARD active_actor == Bronko.talk (related 40)
[593F]     GUARD rec_0350 > 1
[5946]     ENDIF
[5947]     SAY "MURFFALO FACTORY"  '[skip 1]
[5953]     LOADSTR "moskit20.hnm"
[5962]     IF-BLOCK (exit -> @5989)
[5965]       GUARD rec_0350 == 2
[596C]       ENDIF
[596D]       SAY "Me BRONKO ... Me great Butcher Chief on Moskito planet."  '[voice 1]
    END
[5989]     SAY "Me greet you , stranger . You come get murffalo for Corpo ..."  '[voice 2]
[59AB]     SAY "Izwals of Corpo say me you come get murffalo"  '[voice 6]
[59C5]     SAY "You have CRED ? No credit here ..."  '[voice 4]
[59DD]     IF-BLOCK (exit -> @5AB0)
[59E0]       GUARD rec_1018 == 40
[59E5]       ENDIF
[59E6]       SAY "HONK REPORTS :"
[59F4]       SAY "Refuse !!! Commander . Don't tell me we're gonna give this geek a cred to feed rat types ..."
[5A22]       SAY "TELEPORT CRED TO BRONKO word_65535 teleport refuse"
[5A3A]       IF-BLOCK (exit -> @5A59)
[5A3D]         GUARD concept == "teleport"
[5A40]         ENDIF
[5A41]         SAY "CRED TELEPORTED TO BRONKO"  '[skip 2]
[5A51]         OP_CD CD 30 00 04 10 1A 03
[5A58]         CLEAR concept_alt
      END
[5A59]       IF-BLOCK (exit -> @5A9E)
[5A5C]         GUARD concept == "refuse"
[5A5F]         ENDIF
[5A60]         SAY "Good work , Commander ..."
[5A72]         SAY "You waste my time ... Me not have time..."  '[voice 2]
[5A8C]         SAY "Bye bye ..."  '[voice 5, skip 2]
[5A9A]         CLEAR concept_alt
[5A9B]         END PRESENTATION Bronko.talk
      END
[5A9E]       SAY "Thank you , stranger ..."  '[voice 5]
    END
[5AB0]     SAY "What lousy life ... Work always ..."  '[voice 5]
[5AC6]     SAY "One day , them send me to planet Magnus with other worn-out robots ... Lousy life ..."  '[voice 4]
[5AF0]     IF-BLOCK (exit -> @5B53)
[5AF3]       GUARD rec_1018 == 794
[5AF8]       GUARD rec_1060 == 794
[5AFD]       ENDIF
[5AFE]       SAY "Me give you three tons frozen murffalo meat , not more ..."  '[voice 4]
[5B1E]       SAY "TELEPORT MURFFALO TO ARK word_65535 teleport"
[5B34]       IF-BLOCK (exit -> @5B53)
[5B37]         GUARD concept == "teleport"
[5B3A]         ENDIF
[5B3B]         SAY "MURFFALO TELEPORTING TO ARK"  '[skip 2]
[5B4B]         OP_CD CD 54 03 4C 10 28 00
[5B52]         CLEAR concept_alt
      END
    END
[5B53]     IF-BLOCK (exit -> @5C05)
[5B56]       GUARD rec_1018 == 794
[5B5B]       GUARD rec_1060 == 40
[5B60]       ENDIF
[5B61]       SAY "You be loaded ... See you soon , stranger . Come back when want ..."  '[voice 3]
[5B87]       SAY "Bronko like clients with CRED ... Ha! Ha! Ha!"  '[voice 6]
[5BA1]       SAY "Say , Commander , do you notice that smell ? It's that murffalo meat ... What a stink !"
[5BCF]       SAY "Those ratboys must tie a knot in their trunks every time they eat ..."
[5BF3]       SAY "Bye bye , friend ..."  '[voice 4]
    END
[5C05]     IF-BLOCK (exit -> @5C68)
[5C08]       GUARD NOT rec_1018 == 40
[5C0E]       GUARD NOT rec_1018 == 794
[5C14]       ENDIF
[5C15]       SAY "You not have creds ? No creds , no murffalo meat ... Bye bye , stranger .."  '[voice 3]
[5C3F]       SAY "I suggest we find a CRED someplace , Commander ..."
[5C5B]       SAY "stop"  '[skip 1]
[5C65]       END PRESENTATION Bronko.talk
    END
[5C68]     SAY "stop"  '[skip 3]
[5C72]     A1 = 3
[5C79]     F1 = 1
[5C80]     END PRESENTATION Bronko.talk
  END
[5C83]   BLOCK (exit -> @5F31)
[5C87]     AWAIT gameflag_252A
[5C88]     GUARD rec_0F4E == 3818
[5C8D]     GUARD F1 == 1
[5C94]     GUARD active_actor == Bronko.talk (related 40)
[5C99]     ENDIF
[5C9A]     SAY "Ahhh! A visitor ..."  '[voice 5, skip 1]
[5CAA]     adieu = 0
[5CB1]     IF-BLOCK (exit -> @5D0D)
[5CB4]       GUARD rec_0350 == 3
[5CBB]       ENDIF
[5CBC]       SAY "You again , stranger ! What you want ? You come see your friend Bronko ..."  '[voice 2, skip 1]
[5CE4]       rec_0360 = 6566
[5CE9]       SAY "Let's not waste too much time , Commander . The guy's an airhead ..."
    END
[5D0D]     IF-BLOCK (exit -> @5D7F)
[5D10]       GUARD rec_0350 == 4
[5D17]       ENDIF
[5D18]       SAY "You again... SCRUT not eliminated you ? Thousands SCRUT officers on your trail ..."  '[voice 2]
[5D3C]       SAY "He's losing it... A bad case of SCRUT fixation ..."
[5D58]       SAY "You contact me by radio , if you need Bronko ..."  '[voice 4, skip 2]
[5D76]       rec_031C |= 0x2
[5D7B]       POKE [0x5F32] = 1
    END
[5D7F]     IF-BLOCK (exit -> @5EE9)
[5D82]       GUARD rec_0350 > 4
[5D89]       ENDIF
[5D8A]       SAY "Me be alone here... Me hungry of adventure"  '[voice 1]
[5DA2]       SAY "Me think much . Me think be useful for you . You take me with you .."  '[voice 5]
[5DCC]       SAY "You know how to cook ?"
[5DE0]       SAY "Me know make tasty meals..."  '[voice 2]
[5DF2]       SAY "Commander, maybe that's not such a bad idea . He could handle the cooking ..."
[5E18]       SAY "You can teleport me in your ship ..."  '[voice 6, skip 2]
[5E30]       rec_031C &= !0x2
[5E36]       rec_031C |= 0x20
[5E3B]       SAY "TELEPORT BRONKO TO ARK ? word_65535 teleport refuse"
[5E55]       IF-BLOCK (exit -> @5E7A)
[5E58]         GUARD concept == "teleport"
[5E5B]         ENDIF
[5E5C]         SAY "BRONKO TELEPORTING TO ARK"  '[skip 4]
[5E6C]         OP_C2 C2 30 00 1A 03
[5E71]         rec_0842 = 3848
[5E76]         CLEAR concept_alt
[5E77]         END PRESENTATION Bronko.talk
      END
[5E7A]       IF-BLOCK (exit -> @5EE9)
[5E7D]         GUARD concept == "refuse"
[5E80]         ENDIF
[5E81]         SAY "CRY CRY SWEAR ... Me unlucky ... Me all alone ... CRY CRY..."  '[voice 2]
[5EA3]         SAY "Commander ... Get that guy... What a cry baby ... Ha! Ha! Ha!"
[5EC5]         SAY "Me sad ... Me all alone ..."  '[voice 3]
[5EDB]         SAY "..."  '[skip 2]
[5EE5]         CLEAR concept_alt
[5EE6]         END PRESENTATION Bronko.talk
      END
    END
[5EE9]     SAY "Bye bye , friend . Bronko have much lot work ... word_65535 bye_bye"  '[voice 5, skip 1]
[5F0B]     adieu = 1
[5F12]     IF-BLOCK (exit -> @5F31)
[5F15]       GUARD adieu == 1
[5F1C]       ENDIF
[5F1D]       SAY "stop"  '[skip 2]
[5F27]       adieu = -1.value
[5F2E]       END PRESENTATION Bronko.talk
    END
  END
[5F31]   GOTO @5FB2
[5F35]   AWAIT presentation
[5F36]   GUARD rec_0332 == 3848
[5F3B]   GUARD active_actor == Bronko.talk (related 40)
[5F40]   ENDIF
[5F41]   SAY "YES , hello . You be Commander ..."
[5F59]   SAY "Me much lot busy . Me not can talk . SCRUT watch me ..."
[5F7D]   SAY "This Bronko's completely paranoid ..."
[5F8F]   SAY "Me hang up ..."
[5F9F]   SAY "cruikkk!"  '[skip 2]
[5FA9]   rec_031C &= !0x2
[5FAF]   END PRESENTATION Bronko.talk
[5FB2]   BLOCK (exit -> @6084)
[5FB6]     AWAIT gameflag_274F
[5FB7]     GUARD trak8 == 0
[5FBE]     GUARD active_actor == Bronko.talk (related 40)
[5FC3]     ENDIF
[5FC4]     SAY "Nice ship you got , Commander ..."
[5FDA]     SAY "Me do cooking . Me make tasty meals for Cap'n Bob ..."
[5FFA]     SAY "Leave him, Commander. He's cooking ..."  '[skip 1]
[600E]     rec_0360 = 190
[6013]     IF-BLOCK (exit -> @6067)
[6016]       GUARD ekkk == 1
[601D]       GUARD rec_0548 > 0
[6024]       GUARD (rec_0EB0 & 0x2) == 0
[602A]       ENDIF
[602B]       SAY "Ekatomb is at coordinates X657 Y987 ..."  '[voice 3]
[6041]       SAY "Another planet, Commander ... I love this Bronko ..."  '[skip 2]
[605B]       rec_0EB0 |= 0x2
[6060]       trak8 = 1
    END
[6067]     SAY "Me cryonize ... AAAAHHHH!!!"  '[voice 1, skip 3]
[6077]     rec_082C |= 0x1
[607C]     rec_0842 = 3848
[6081]     END PRESENTATION Bronko.talk
  END
[6084]   BLOCK (exit -> @6109)
[6088]     AWAIT gameflag_274F
[6089]     GUARD active_actor == Bronko.talk (related 40)
[608E]     GUARD trak8 == 1
[6095]     ENDIF
[6096]     SAY "Me busy do onboard cooking . Me make tasty meals for Cap'n Bob ..."
[60BA]     SAY "Let him do his job , Commander . It's about time we had something edible to eat ..."
[60E6]     SAY "Me not have time , Commander ..."
[60FC]     SAY "..."  '[skip 1]
[6106]     END PRESENTATION Bronko.talk
  END
[6109]   BLOCK (exit -> @641F)
[610D]     AWAIT gameflag_252A
[610E]     GUARD active_actor == Emasculator.talk (related 40)
[6113]     GUARD rec_0F4E == 3818
[6118]     GUARD X1 == 0
[611F]     ENDIF
[6120]     SAY "What you want , stranger ? You want murffalo meat , me new seller ..."  '[voice 1]
[6146]     SAY "You have CRED to pay ?"  '[voice 2]
[615A]     SAY "No cred , no murffalo meat ..."  '[voice 4]
[6170]     SAY "So that's Bronko's replacement in the meat-selling business ..."
[618A]     SAY "You have CRED ? No credit here ..."  '[voice 4]
[61A2]     IF-BLOCK (exit -> @6244)
[61A5]       GUARD rec_1018 == 40
[61AA]       ENDIF
[61AB]       SAY "TELEPORT CRED TO EMASCULATOR word_65535 teleport refuse"
[61C3]       IF-BLOCK (exit -> @61FF)
[61C6]         GUARD concept == "teleport"
[61C9]         ENDIF
[61CA]         SAY "CRED TELEPORTED TO EMASCULATOR"  '[skip 2]
[61DA]         OP_CD CD 30 00 04 10 1A 03
[61E1]         PP1 = 1
[61E8]         SAY "Thank you , stranger Ha! Ha! Ha!..."  '[voice 5, skip 1]
[61FE]         CLEAR concept_alt
      END
[61FF]       IF-BLOCK (exit -> @6244)
[6202]         GUARD concept == "refuse"
[6205]         ENDIF
[6206]         SAY "Good work , Commander ..."
[6218]         SAY "You waste my time ... Me not have time..."  '[voice 2]
[6232]         SAY "Bye bye ..."  '[voice 5, skip 2]
[6240]         CLEAR concept_alt
[6241]         END PRESENTATION Emasculator.talk
      END
    END
[6244]     IF-BLOCK (exit -> @6255)
[6247]       GUARD NOT rec_11C8 == 40
[624D]       ENDIF
[624E]       bion = 1
    END
[6255]     IF-BLOCK (exit -> @633B)
[6258]       GUARD bion == 1
[625F]       ENDIF
[6260]       SAY "You be adventurer ... Me know much things ..."  '[voice 6]
[627A]       SAY "Me say you this ... ME MUCH NEED BIONIUM ..."  '[voice 2]
[6296]       SAY "You bring BIONIUM , me pay much ..."  '[voice 3]
[62AE]       SAY "Commander , Commander , a client !"
[62C4]       SAY "Me give you free container ..."  '[voice 4]
[62D8]       SAY "What !!! A whole container ... Opportunity knocks , Commander ..."
[62F6]       SAY "TELEPORT BIONIUM CONTAINER TO ARK word_65535 teleport"
[630E]       IF-BLOCK (exit -> @633B)
[6311]         GUARD concept == "teleport"
[6314]         ENDIF
[6315]         SAY "CONTAINER TELEPORTED TO ARK"  '[skip 4]
[6325]         OP_CD CD 64 08 B4 11 28 00
[632C]         trak9 = 1
[6333]         bion = 0
[633A]         CLEAR concept_alt
      END
    END
[633B]     IF-BLOCK (exit -> @63F2)
[633E]       GUARD vbio > 0
[6345]       GUARD rec_11C8 == 40
[634A]       GUARD rec_0860 > 1
[6351]       ENDIF
[6352]       SAY "Give me your BIONIUM stanger..."  '[voice 0]
[6364]       SAY "TELEPORT BIONIUM TO EMASCULATOR word_65535 teleport refuse"
[637C]       IF-BLOCK (exit -> @63D6)
[637F]         GUARD concept == "teleport"
[6382]         ENDIF
[6383]         SAY "BIONIUM TELEPORTED TO EMASCULATOR"  '[skip 2]
[6393]         vbio = 1
[639A]         LOADSTR "star2.hnm"
[63A6]         SAY "GOOD GOOD... Me much work now ... You waste my time ..."  '[voice 4]
[63C6]         SAY "Bye bye..."  '[voice 4, skip 2]
[63D2]         CLEAR concept_alt
[63D3]         END PRESENTATION Emasculator.talk
      END
[63D6]       IF-BLOCK (exit -> @63F2)
[63D9]         GUARD concept == "refuse"
[63DC]         ENDIF
[63DD]         SAY "Ok Commander , you're the boss..."  '[skip 1]
[63F1]         CLEAR concept_alt
      END
    END
[63F2]     SAY "Me much work ... You not waste my time ..."  '[voice 4]
[640E]     SAY "Bye bye stranger"  '[skip 1]
[641C]     END PRESENTATION Emasculator.talk
  END
[641F]   BLOCK (exit -> @650A)
[6423]     AWAIT gameflag_252A
[6424]     GUARD rec_0F4E == 3074
[6429]     GUARD B1 == 0
[6430]     GUARD active_actor == Morning_Oil.talk (related 40)
[6435]     ENDIF
[6436]     SAY "Some player shoots again ... Rem RLA LDIR ... POP AX ... Mov bx,19 ... deltree."  '[voice 2, skip 1]
[645E]     rec_02D0 = 11262
[6463]     SAY "COMPUTING ENTITY HONK REPORTS AT THIS TIME :"
[647B]     SAY "Commander , that's a droid ... A robot from the last war ..."
[649D]     SAY "He's broken down . Keep him talking , while I run a few tests ..."
[64C3]     SAY "Rem dir Fatal Error ..."  '[voice 2]
[64D5]     SAY "Hello there ... Ahhhh... word_65535 bye_bye"  '[skip 1]
[64E9]     adieu = 1
[64F0]     IF-BLOCK (exit -> @650A)
[64F3]       GUARD adieu == 1
[64FA]       ENDIF
[64FB]       POKE [0x650B] = 1
[64FF]       adieu = -1.value
[6506]       POKE [0x6420] = 0
    END
  END
[650A]   GOTO @65E6
[650E]   AWAIT gameflag_252A
[650F]   GUARD B1 == 0
[6516]   ENDIF
[6517]   SAY "HONK HERE :"
[6525]   SAY "Commander, I'd kinda like to unscrew a few things and take a look inside the droid ..."
[654F]   SAY "His batteries seem flat to me ..."
[6565]   SAY "cruik cruiik ........... BZZZ..."
[6575]   SAY "Better get some batteries fast , Commander!"
[658B]   SAY "We could use this robot ..."
[659F]   SAY "Hello there ...........krrak.. word_65535 bye_bye"  '[skip 1]
[65B1]   adieu = 1
[65B8]   IF-BLOCK (exit -> @65E6)
[65BB]     GUARD adieu == 1
[65C2]     ENDIF
[65C3]     B1 = 1
[65CA]     B11 = 1
[65D1]     trak13 = 1
[65D8]     adieu = -1.value
[65DF]     POKE [0x650B] = 0
[65E3]     END PRESENTATION Morning_Oil.talk
  END
[65E6]   BLOCK (exit -> @66F5)
[65EA]     AWAIT gameflag_252A
[65EB]     GUARD rec_0F4E == 3074
[65F0]     GUARD B1 == 1
[65F7]     GUARD active_actor == Morning_Oil.talk (related 40)
[65FC]     ENDIF
[65FD]     IF-BLOCK (exit -> @6653)
[6600]       GUARD rec_11E0 == 40
[6605]       ENDIF
[6606]       SAY "khjhkjsd krrak."  '[voice 3]
[6612]       SAY "HURRY , Commander ... Teleport him the batteries... word_65535 teleport"
[6630]       IF-BLOCK (exit -> @6653)
[6633]         GUARD concept == "teleport"
[6636]         ENDIF
[6637]         SAY "TELEPORTING BATTERIES TO MORNING OIL ROBOT"  '[skip 2]
[664B]         OP_CD CD 30 00 CC 11 8A 02
[6652]         CLEAR concept_alt
      END
    END
[6653]     IF-BLOCK (exit -> @66F5)
[6656]       GUARD rec_11E0 == 650
[665B]       ENDIF
[665C]       rec_028C |= 0x20
[6661]       SAY "Looks like it's working , Commander . His brain's emitting alpha waves ..."
[6683]       SAY "We better get him on board the Ark , Commander . I'll need to open him up ... word_65535 teleport"
[66B5]       IF-BLOCK (exit -> @66D4)
[66B8]         GUARD concept == "teleport"
[66BB]         ENDIF
[66BC]         SAY "TELEPORT MORNING OIL TO CRYOBOX"  '[skip 2]
[66CE]         OP_C2 C2 30 00 8A 02
[66D3]         CLEAR concept_alt
      END
[66D4]       IF-BLOCK (exit -> @66F5)
[66D7]         GUARD rec_02A2 == 65535
[66DC]         ENDIF
[66DD]         B1 = 2
[66E4]         B11 = -1.value
[66EB]         trak14 = 1
[66F2]         END PRESENTATION Morning_Oil.talk
      END
    END
  END
[66F5]   BLOCK (exit -> @6775)
[66F9]     AWAIT gameflag_252A
[66FA]     GUARD rec_0F4E == 3074
[66FF]     GUARD B11 == 1
[6706]     GUARD NOT rec_11E0 == 40
[670C]     GUARD NOT rec_11E0 == 650
[6712]     GUARD active_actor == Morning_Oil.talk (related 40)
[6717]     ENDIF
[6718]     SAY "RLA RLA LDIR PUSH A,X POP POP HJJKLMDFD KLML HL HFFGFDGHFGJ J KL GJKHL JKLL JG"  '[voice 1]
[6740]     SAY "HONK: We ought to get some batteries , Commander . This is an interesting robot ..."
[6768]     SAY "stop"  '[skip 1]
[6772]     END PRESENTATION Morning_Oil.talk
  END
[6775]   BLOCK (exit -> @68BA)
[6779]     AWAIT gameflag_252A
[677A]     GUARD viol == 1
[6781]     GUARD active_actor == Morning_Oil.talk (related 40)
[6786]     ENDIF
[6787]     SAY "What's that he's saying ? Honk! Behave yourself !!!"  '[voice 6]
[67A1]     SAY "I'm not deaf ... Watch your neurons . Next time , I'll short-circuit you ..."  '[voice 5]
[67C7]     SAY "See , Commander ! It's what I thought ... He's getting violent ..."
[67E9]     SAY "Me violent ?? He's the one who's making trouble . I'm perfectly calm ..."  '[voice 3]
[680D]     SAY "OUCH !!! Commander , he just gave me an electric shock ! Ow ! Hey , cut that out , will ya ! ..."
[6845]     SAY "Serves you right ! That's what happens when you mess with me ..."  '[voice 5]
[6867]     SAY "Commander , why don't I switch him off , huh ? Commander ... OUCH ! He did it again ..."
[6897]     SAY "Okay okay . I'll leave you alone ..."  '[skip 2]
[68AF]     POKE [0x6776] = 0
[68B3]     viol = 0
  END
[68BA]   BLOCK (exit -> @6A7A)
[68BE]     AWAIT gameflag_252A
[68BF]     GUARD caches == 1
[68C6]     GUARD active_actor == Morning_Oil.talk (related 40)
[68CB]     ENDIF
[68CC]     SAY "What's that he's saying ? I don't have clearance for that information !!! !!! !!!"
[68F2]     SAY "That's outrageous !! We better wake up Cap'n Bob right now ..."
[6912]     SAY "Switch him off , Commander ... word_65535 disconnect refuse"  '[voice 3]
[692E]     IF-BLOCK (exit -> @6A49)
[6931]       GUARD concept == "disconnect"
[6934]       ENDIF
[6935]       SAY "No !!! Commander ..."
[6945]       SAY "AAAAAAAAAAAAAAAAAAA ..."
[6951]       SAY "Good work , Commander . You made a wise decision ... Hee! Hee! Hee!"  '[voice 4]
[6975]       SAY "But I don't know anything about that hiding-place , Commander . I just wanted to annoy Honk !!!"  '[voice 5]
[69A1]       SAY "Let's switch him back on ... He's gonna be real mad ... Hee! Hee! Hee!"  '[voice 1]
[69C7]       SAY "So what did he tell you ? ... A pack of lies , believe me ... SWEAR , INSULT"
[69F5]       SAY "Mister Honk ought to mind his language . Signed Olga."
[6A11]       SAY "Olga ! Why don't you shut up !!! I'm a very angry onboard computing facility , Commander ..."  '[skip 3]
[6A3D]       caches = 0
[6A44]       POKE [0x68BB] = 0
[6A48]       CLEAR concept_alt
    END
[6A49]     IF-BLOCK (exit -> @6A7A)
[6A4C]       GUARD concept == "refuse"
[6A4F]       ENDIF
[6A50]       SAY "Too bad . I won't tell you anything at all !!"  '[voice 2, skip 3]
[6A6E]       caches = 0
[6A75]       POKE [0x68BB] = 0
[6A79]       CLEAR concept_alt
    END
  END
[6A7A]   BLOCK (exit -> @6D40)
[6A7E]     AWAIT gameflag_274F
[6A7F]     GUARD rec_02A2 == 65535
[6A84]     GUARD B1 == 2
[6A8B]     GUARD rec_11E0 == 650
[6A90]     ENDIF
[6A91]     SAY "Ahhh ... Morning Oil , veteran of the Great Croolis War , reporting for duty. I greet you and say thank you also..."  '[voice 1]
[6AC7]     SAY "You gave me back my life ... Ah! I feel quite lubrified , I must say ! Recharged batteries too, eh ?"  '[voice 2]
[6AFB]     SAY "My gratitude knows no bounds , noble ones . And I just love your ship . Not a model I'm familiar with ..."  '[voice 3]
[6B31]     SAY "Ahh , if only you'd seen the SPIDER4000 I piloted ... Such invigorating combat ... BABOOOM... VRRRRRRR CHAKA CHAKA CHAKA ..."  '[voice 4]
[6B63]     SAY "BIOCONSCIOUSNESS HONK REPORTS :"
[6B73]     SAY "Commander, this is one deranged robot we got here . I hate him ..."
[6B97]     SAY "We fired at everything that moved , naturally ... Luckily , we managed to hide the treasure before the end ..."  '[voice 5]
[6BC9]     SAY "And my revered master, the splendid Croolis EVISCERATOR was alas made prisoner ..."  '[voice 3]
[6BEB]     SAY "He knows where the Croolis war treasure is hidden ..."  '[voice 4]
[6C07]     SAY "HONK REPORTS :"
[6C15]     SAY "I think I'll just open him up and check that story ..."
[6C35]     SAY "He's bluffing , Commander... I suppose you want me to switch him off now ? word_65535 yes no"
[6C63]     IF-BLOCK (exit -> @6CD5)
[6C66]       GUARD concept == "no"
[6C69]       ENDIF
[6C6A]       SAY "NO , YOU'RE OUT OF YOUR MIND ... I DON'T WANT THIS TO HAPPEN ... MOTHER , HELP !"  '[voice 6]
[6C98]       SAY "HONK MAKES HIS REPORT :"
[6CAA]       SAY "Commander , I think I'll switch him off anyway ... His program needs to be checked ..."  '[skip 1]
[6CD4]       CLEAR concept_alt
    END
[6CD5]     IF-BLOCK (exit -> @6D1B)
[6CD8]       GUARD concept == "yes"
[6CDB]       ENDIF
[6CDC]       SAY "NO , YOU'RE OUT OF YOUR MIND ... I DON'T WANT THIS TO HAPPEN ... MOTHER , HELP !"  '[voice 6]
[6D0A]       SAY "HONK:Switch-off procedure activated ...."  '[skip 1]
[6D1A]       CLEAR concept_alt
    END
[6D1B]     SAY "AAAAAAAAAAAAAAA..."
[6D25]     SAY "stop"  '[skip 3]
[6D2F]     trak15 = 1
[6D36]     B1 = 4
[6D3D]     END PRESENTATION Morning_Oil.talk
  END
[6D40]   BLOCK (exit -> @6DBD)
[6D44]     GUARD B1 == 4
[6D4B]     AWAIT gameflag_274F
[6D4C]     GUARD active_actor == Morning_Oil.talk (related 40)
[6D51]     ENDIF
[6D52]     SAY "Aaaaaah..."  '[voice 2, skip 2]
[6D5C]     state[6] = 10
[6D60]     rec_02D0 = 12304
[6D65]     IF-BLOCK (exit -> @6DBD)
[6D68]       GUARD state[6] == 0
[6D6A]       ENDIF
[6D6B]       SAY "I love it when they snore like that ... Leave him to me , Commander . I'll do a GARBAGE RECOVERY on his memory ..."
[6DA5]       SAY "stop"  '[skip 3]
[6DAF]       B1 = 5
[6DB6]       state[6] = 65535
[6DBA]       END PRESENTATION Morning_Oil.talk
    END
  END
[6DBD]   BLOCK (exit -> @6EAD)
[6DC1]     GUARD B1 == 5
[6DC8]     GUARD active_actor == Morning_Oil.talk (related 40)
[6DCD]     AWAIT gameflag_274F
[6DCE]     ENDIF
[6DCF]     SAY "THE HONK REPORT :"
[6DDF]     SAY "You're gonna love this , Commander. I put the robot's principal memories together ..."
[6E03]     SAY "You can watch them on TV channel two ..."  '[skip 1]
[6E1D]     SETCHAR slot 2 = "match"
[6E26]     SAY "That droid has seen some weird stuff , Commander . That hidden treasure story could be true . We'll have to find that planet ."
[6E60]     SAY "Give me a little time ... I have to change a couple of blown fuses ..."
[6E88]     SAY "..."
[6E92]     SAY "stop"  '[skip 3]
[6E9C]     trak16 = 1
[6EA3]     B1 = 6
[6EAA]     END PRESENTATION Morning_Oil.talk
  END
[6EAD]   BLOCK (exit -> @7033)
[6EB1]     AWAIT gameflag_274F
[6EB2]     GUARD B1 == 6
[6EB9]     GUARD active_actor == Morning_Oil.talk (related 40)
[6EBE]     ENDIF
[6EBF]     SAY "Commander , Commander , Morning Oil has something big , big , big to tell you ... Commander ..."
[6EED]     SAY "Commander , I know a way to make us a pile of CREDs : BIONIUM ..."  '[voice 2]
[6F15]     SAY "BIONIUM is an energy substance found in Cyberspace pockets ..."  '[voice 3]
[6F31]     SAY "If we could get hold of a BIONIUM container , then all we'd have to do is enter Cyberspace to collect the stuff ..."  '[voice 4]
[6F69]     SAY "BIONIUM sells for big money , Commander . It's the power source for all robots ..."  '[voice 5]
[6F91]     SAY "This could be a major breakthrough , Commander . All we need is a BIONIUM container ..."  '[skip 1]
[6FBB]     rec_02D0 = 12464
[6FC0]     SAY "Commander, I reckon ol'Morning Oil's operational,"
[6FD4]     SAY "I'll tell him to repaint the ARK's hull. The old girl's getting rusty..."
[6FF6]     SAY "You can talk with Morning Oil clicking on the ARK position..."
[7014]     SAY "Bye bye Commander ..."  '[voice 0, skip 3]
[7024]     rec_02A2 = 3932
[7029]     B1 = 7
[7030]     END PRESENTATION Morning_Oil.talk
  END
[7033]   BLOCK (exit -> @7147)
[7037]     AWAIT gameflag_252A
[7038]     GUARD B1 == 7
[703F]     GUARD rec_02A2 == 3932
[7044]     GUARD (rec_09E8 & 0x2) == 0
[704A]     GUARD active_actor == Morning_Oil.talk (related 40)
[704F]     ENDIF
[7050]     SAY "HONK REPORTS HERE AND NOW :"
[7064]     SAY "Commander, I cleaned up his memories . You can question him now ."  '[skip 1]
[7086]     rec_02D0 = 12464
[708B]     SAY "MASTACHOK... word_65535 mastachok"  '[voice 2, skip 1]
[7099]     mas = 1
[70A0]     IF-BLOCK (exit -> @7134)
[70A3]       GUARD mas == 1
[70AA]       ENDIF
[70AB]       SAY "I know the coordinates for the planet Mastachok , where my splendid master EVISCERATOR is imprisoned ..."  '[voice 4]
[70D5]       SAY "HONK:"
[70DF]       SAY "Commander , it's what I thought ... This robot's been holding out on us ..."  '[skip 1]
[7105]       trak17 = 1
[710C]       SAY "Mastachok is in the Galabar X232 constellation ..."  '[voice 5, skip 3]
[7124]       rec_09E8 |= 0x2
[7129]       mas = 0
[7130]       POKE [0x7148] = 1
    END
[7134]     SAY "Bye bye word_65535 bye_bye"  '[skip 1]
[7144]     END PRESENTATION Morning_Oil.talk
  END
[7147]   GOTO @718C
[714B]   AWAIT gameflag_252A
[714C]   GUARD B1 == 7
[7153]   GUARD rec_02A2 == 3932
[7158]   GUARD (rec_09E8 & 0x2) != 0
[715D]   GUARD active_actor == Morning_Oil.talk (related 40)
[7162]   ENDIF
[7163]   SAY "Cruiiiiiikkk!"  '[voice 2]
[716D]   SAY "..."  '[skip 4]
[7177]   C1 = 1
[717E]   B1 = 8
[7185]   POKE [0x7148] = 0
[7189]   END PRESENTATION Morning_Oil.talk
[718C]   BLOCK (exit -> @722D)
[7190]     AWAIT gameflag_252A
[7191]     GUARD rec_02A2 == 3932
[7196]     GUARD (rec_09E8 & 0x2) != 0
[719B]     GUARD B1 == 8
[71A2]     GUARD active_actor == Morning_Oil.talk (related 40)
[71A7]     ENDIF
[71A8]     SAY "Sorry , Commander , but I was just doing some minor repairs on Morning Oil ..."
[71D0]     SAY "He's not ready to talk right now ..."
[71E8]     SAY "But he can continue to repaint the ARK's hull. The old girl's getting rusty..."
[720C]     SAY "Sorry about that , Commander ..."  '[voice 1]
[7220]     SAY "..."  '[skip 1]
[722A]     END PRESENTATION Morning_Oil.talk
  END
[722D]   BLOCK (exit -> @7254)
[7231]     GUARD C1 == 1
[7238]     GUARD rec_02A2 == 3932
[723D]     GUARD rec_0332 == 65535
[7242]     GUARD NOT rec_1030 == 40
[7248]     GUARD trak7 == 1
[724F]     ENDIF
[7250]     POKE [0x7255] = 0
  END
[7254]   BLOCK (exit -> @7413)
[7258]     AWAIT gameflag_252A
[7259]     GUARD rec_0F4E == 2534
[725E]     GUARD active_actor == Scruter_Mac.talk (related 40)
[7263]     GUARD rec_06B0 == 1
[726A]     ENDIF
[726B]     SAY "Yikes ... A guard ..."
[727D]     SAY "FORBIDDEN ZONE YOU GIVE PASSWORD : word_65535 djerk twist mashpotatoes madison locomoshun"  '[voice 5]
[729F]     IF-BLOCK (exit -> @72C9)
[72A2]       GUARD concept == "djerk"
[72A5]       ENDIF
[72A6]       SAY "Djerk not be code . Djerk be exciting dance step , friends ..."  '[voice 6, skip 1]
[72C8]       CLEAR concept_alt
    END
[72C9]     IF-BLOCK (exit -> @72F3)
[72CC]       GUARD concept == "locomoshun"
[72CF]       ENDIF
[72D0]       SAY "Locomoshun not be code . Be dance everybody be doing , friends ..."  '[voice 6, skip 1]
[72F2]       CLEAR concept_alt
    END
[72F3]     IF-BLOCK (exit -> @731F)
[72F6]       GUARD concept == "twist"
[72F9]       ENDIF
[72FA]       SAY "Twist not be code . Twist be dance bad for back , friends ..."  '[voice 6, skip 1]
[731E]       CLEAR concept_alt
    END
[731F]     IF-BLOCK (exit -> @7345)
[7322]       GUARD concept == "mashpotatoes"
[7325]       ENDIF
[7326]       SAY "Mashpotatoes not be code. Be dance for potato , friends ..."  '[voice 6, skip 1]
[7344]       CLEAR concept_alt
    END
[7345]     IF-BLOCK (exit -> @736D)
[7348]       GUARD concept == "madison"
[734B]       ENDIF
[734C]       SAY "Madison not be code. Madison be exotic dance routine , friends ..."  '[voice 6, skip 1]
[736C]       CLEAR concept_alt
    END
[736D]     SAY "Commander, you're hopeless at this.... We need to find that stupid code ..."
[738F]     SAY "You not found code . You be spy . Me exterminate you now ...."  '[voice 10]
[73B3]     SAY "He's gonna zap the Orxx ..."
[73C7]     SAY "You should've listened to me , Commander... I'll have to tell Cap'n Bob ... Sorry ..."
[73EF]     SAY "Baaaoooommmm!!!!"  '[voice 5, skip 1]
[73F9]     LOADSTR "explo3.hnm"
[7406]     SAY "stop"  '[skip 1]
[7410]     END PRESENTATION Scruter_Mac.talk
  END
[7413]   BLOCK (exit -> @764F)
[7417]     AWAIT gameflag_252A
[7418]     GUARD C1 == 1
[741F]     GUARD NOT rec_1030 == 40
[7425]     GUARD rec_0F4E == 2534
[742A]     GUARD active_actor == Scruter_Mac.talk (related 40)
[742F]     ENDIF
[7430]     SAY "SCANNING STRANGER...XRAY....STOP"  '[voice 0, skip 1]
[743C]     rec_06C0 = 1
[7441]     SAY "ZONE ULTRAFORBIDDEN YOU NOT STAY HERE ... YOU GO AWAY ..."  '[voice 1]
[745F]     SAY "FIRST WARNING ..."  '[voice 1]
[746D]     IF-BLOCK (exit -> @74C2)
[7470]       GUARD rec_06B0 == 1
[7477]       ENDIF
[7478]       SAY "Oh no ! Him again ... Commander , I don't feel happy about that SCRUT robot ..."
[74A2]       SAY "We'll need to be smart , cunning , intelligent and clever ..."
    END
[74C2]     IF-BLOCK (exit -> @74ED)
[74C5]       GUARD rec_06B0 == 2
[74CC]       ENDIF
[74CD]       SAY "Watch out , Commander ... Second try ... Don't blow it ..."
    END
[74ED]     IF-BLOCK (exit -> @751A)
[74F0]       GUARD rec_06B0 == 3
[74F7]       ENDIF
[74F8]       SAY "I think I found the code , Commander... It's the number 9 ..."
    END
[751A]     SAY "You give identity code : word_65535 code password ole 3 4 5 6 7 8 9"  '[voice 1]
[7544]     IF-BLOCK (exit -> @75DB)
[7547]       GUARD concept == "code"
[754A]       ENDIF
[754B]       SAY "Yes ! Code be CODE. You be very Important Being ..."  '[voice 2]
[7569]       SAY "Nice work , Commmander , I've got to hand it to you ... How did you figure it out ?"
[7599]       SAY "Ol' Turkey Face Bob'll be proud of you when I tell him ..."
[75BB]       SAY "You did give code . You be authorized ..."  '[voice 1, skip 2]
[75D5]       rec_06C0 = 1
[75DA]       CLEAR concept_alt
    END
[75DB]     IF-BLOCK (exit -> @7637)
[75DE]       GUARD NOT concept == "code"
[75E2]       ENDIF
[75E3]       SAY "You unauthorized . Me exterminate you now ..."  '[voice 3]
[75FB]       SAY "Uh oh Commander . He's gonna zap the Orxx..."
[7615]       SAY "Bye bye , unauthorized strangers ..."  '[voice 4]
[7629]       SAY "Stop"  '[skip 2]
[7633]       END PRESENTATION Scruter_Mac.talk
[7636]       CLEAR concept_alt
    END
[7637]     SAY "stop word_65535 bye_bye"  '[skip 2]
[7645]     C1 = 2
[764C]     END PRESENTATION Scruter_Mac.talk
  END
[764F]   BLOCK (exit -> @778B)
[7653]     AWAIT gameflag_252A
[7654]     GUARD C1 == 2
[765B]     GUARD NOT rec_1030 == 40
[7661]     GUARD active_actor == Scruter_Mac.talk (related 40)
[7666]     ENDIF
[7667]     SAY "HALT STRANGER . ME HAVE THING TO SAY ..."  '[voice 6]
[7681]     SAY "You give identity code : word_65535 code 125 242 44 5 666 70 800"  '[voice 1]
[76A7]     IF-BLOCK (exit -> @76E3)
[76AA]       GUARD concept == "code"
[76AD]       ENDIF
[76AE]       SAY "You did give code . You be authorized . Me know you , handsome stranger ..."  '[voice 1, skip 3]
[76D6]       PP1 = 1
[76DD]       rec_06C0 = 167
[76E2]       CLEAR concept_alt
    END
[76E3]     IF-BLOCK (exit -> @7773)
[76E6]       GUARD NOT concept == "code"
[76EA]       ENDIF
[76EB]       SAY "You unauthorized . Me exterminate you now ..."  '[voice 3]
[7703]       SAY "You really fouled up there , Commander . The code was code ... You knew that !"
[772D]       SAY "What a dummy ... Cap'n Bob won't like it when I tell him !"
[7751]       SAY "Bye bye , unauthorized strangers ..."  '[voice 4]
[7765]       SAY "Stop"  '[skip 2]
[776F]       END PRESENTATION Scruter_Mac.talk
[7772]       CLEAR concept_alt
    END
[7773]     SAY "stop word_65535 bye_bye"  '[skip 2]
[7781]     C1 = 3
[7788]     END PRESENTATION Scruter_Mac.talk
  END
[778B]   BLOCK (exit -> @78EF)
[778F]     AWAIT gameflag_252A
[7790]     GUARD C1 == 3
[7797]     GUARD NOT rec_1030 == 40
[779D]     GUARD active_actor == Scruter_Mac.talk (related 40)
[77A2]     ENDIF
[77A3]     SAY "HALT STRANGER . YOU SAY CODE.."  '[voice 6]
[77B7]     SAY "Hurry , stranger ! word_65535 code 1 2 3 4 5 6 7 8 9 0"  '[voice 1]
[77E1]     IF-BLOCK (exit -> @7832)
[77E4]       GUARD concept == "code"
[77E7]       ENDIF
[77E8]       SAY "You did say code . You be authorized . Me recognize you ."  '[voice 12]
[780A]       SAY "You come often ... Me suspicious of you ... Me getting trigger-happy ..."  '[voice 0, skip 2]
[782C]       rec_06C0 = 13066
[7831]       CLEAR concept_alt
    END
[7832]     IF-BLOCK (exit -> @78D7)
[7835]       GUARD NOT concept == "code"
[7839]       ENDIF
[783A]       SAY "You unauthorized ... Me exterminate you now ..."  '[voice 3]
[7852]       SAY "Ouch ! That was the wrong code , Commander . The right code was code . I thought you knew that ..."
[7886]       SAY "Cap'n Bob won't be happy when I tell him about this failure ...!"
[78A8]       SAY "Bye bye , unauthorized strangers ..."  '[voice 4, skip 1]
[78BC]       LOADSTR "explo3.hnm"
[78C9]       SAY "Stop"  '[skip 2]
[78D3]       END PRESENTATION Scruter_Mac.talk
[78D6]       CLEAR concept_alt
    END
[78D7]     SAY "stop word_65535 bye_bye"  '[skip 2]
[78E5]     C1 = 4
[78EC]     END PRESENTATION Scruter_Mac.talk
  END
[78EF]   BLOCK (exit -> @7974)
[78F3]     AWAIT gameflag_252A
[78F4]     GUARD C1 == 4
[78FB]     GUARD NOT rec_1030 == 40
[7901]     GUARD NOT rec_1030 == 1658
[7907]     GUARD active_actor == Scruter_Mac.talk (related 40)
[790C]     ENDIF
[790D]     SAY "You irritate me ..."  '[voice 6, skip 1]
[791D]     PP1 = 1
[7924]     SAY "You go away . Me suspicious of you ..."  '[voice 5]
[793E]     SAY "Me liquidize you ..."  '[voice 4]
[794E]     SAY "BBBBAAAAAAAAOOOMMMM !!!!!!!!!!!"  '[voice 5, skip 1]
[795A]     LOADSTR "explo3.hnm"
[7967]     SAY "stop"  '[skip 1]
[7971]     END PRESENTATION Scruter_Mac.talk
  END
[7974]   BLOCK (exit -> @798A)
[7978]     GUARD rec_0F4E == 2534
[797D]     GUARD rec_1030 == 40
[7982]     ENDIF
[7983]     C1 = 5
  END
[798A]   BLOCK (exit -> @7A44)
[798E]     AWAIT gameflag_252A
[798F]     GUARD C1 == 5
[7996]     GUARD rec_0F4E == 2534
[799B]     GUARD rec_1030 == 40
[79A0]     GUARD active_actor == Scruter_Mac.talk (related 40)
[79A5]     ENDIF
[79A6]     SAY "You again , stranger ..."  '[voice 5]
[79B8]     SAY "You smell real nice ... Me like your perfume ..."  '[voice 4]
[79D4]     SAY "You want see prisoner ..."  '[voice 6]
[79E6]     SAY "Commander, give him that perfume . The ship stinks ... word_65535 teleport"
[7A08]     IF-BLOCK (exit -> @7A44)
[7A0B]       GUARD concept == "teleport"
[7A0E]       ENDIF
[7A0F]       SAY "TELEPORT PERFUME TO SCRUT ROBOT MAC"
[7A23]       SAY "Nifty work , Commander ..."  '[skip 3]
[7A35]       OP_CD CD 30 00 1C 10 7A 06
[7A3C]       parf = 1
[7A43]       CLEAR concept_alt
    END
  END
[7A44]   BLOCK (exit -> @7B46)
[7A48]     AWAIT gameflag_252A
[7A49]     GUARD C1 == 5
[7A50]     GUARD rec_0F4E == 2534
[7A55]     GUARD rec_1030 == 1658
[7A5A]     GUARD parf == 1
[7A61]     GUARD active_actor == Scruter_Mac.talk (related 40)
[7A66]     ENDIF
[7A67]     SAY "Oooh! Present for me ! I be so moved ..."  '[voice 5]
[7A83]     SAY "Me undo package quick ................"  '[voice 6]
[7A95]     SAY "Ooooooooohh! Big thanks ... Me like perfume ... You be friend ..."  '[voice 5]
[7AB5]     SAY "Perfume , Exhaust Nozzle Romeo ... Me like ... Me like ..."  '[voice 6]
[7AD5]     SAY "You want see Croolis prisoner , friend ? You come back soon ... We wait you ..."  '[voice 7]
[7AFF]     SAY "Not can see prisoner now . Prisoner be in comfort station ... You come back , friend ..."  '[voice 2]
[7B2B]     SAY "stop"  '[skip 3]
[7B35]     parf = 0
[7B3C]     C1 = 6
[7B43]     END PRESENTATION Scruter_Mac.talk
  END
[7B46]   BLOCK (exit -> @7C11)
[7B4A]     AWAIT gameflag_252A
[7B4B]     GUARD C1 == 6
[7B52]     GUARD rec_0F4E == 2534
[7B57]     GUARD rec_1030 == 1658
[7B5C]     GUARD active_actor == Scruter_Mac.talk (related 40)
[7B61]     ENDIF
[7B62]     SAY "Oooh! You be back ... Me perfumed . You smell me ..."  '[voice 5]
[7B82]     SAY "Me soon go dance tekno ..."  '[voice 6]
[7B96]     SAY "You want see Croolis prisoner , friend ."  '[voice 7]
[7BAE]     SAY "Not can see prisoner now . Prisoner still in comfort station ..."  '[voice 2]
[7BCE]     SAY "You must be patient . Come back later ..."  '[voice 3]
[7BE8]     SAY "Bye bye , friend . Me must stand guard ..."  '[voice 1]
[7C04]     SAY "stop"  '[skip 1]
[7C0E]     END PRESENTATION Scruter_Mac.talk
  END
[7C11]   BLOCK (exit -> @7DE1)
[7C15]     AWAIT gameflag_252A
[7C16]     GUARD rec_0F4E == 2858
[7C1B]     GUARD active_actor == Yoko.talk (related 40)
[7C20]     GUARD H1 == 0
[7C27]     ENDIF
[7C28]     SAY "Hello , stranger . Me be Yoko, me izwal . Welcome to planet Rondo ..."  '[voice 3]
[7C4E]     SAY "Yikes ! How ugly can you get , Commander ? Get a load of those ears ..."  '[skip 2]
[7C78]     secret = 0
[7C7F]     adieu = 0
[7C86]     SAY "Me greet you , noble stranger ..."  '[voice 3, skip 1]
[7C9C]     rec_0240 = 6
[7CA1]     SAY "You want talk ?"  '[voice 4]
[7CB1]     IF-BLOCK (exit -> @7D5F)
[7CB4]       GUARD secret == 1
[7CBB]       GUARD (rec_0EB0 & 0x2) == 0
[7CC1]       ENDIF
[7CC2]       SAY "Me know Slimers . Them live on nice planet Ekatomb, in jelly village ."  '[voice 6]
[7CE6]       SAY "Them be GLUXX family ... Them be very nice ..."  '[voice 9]
[7D02]       SAY "Planet Ekatomb be in EKATOMBUS 435DRTX constellation ..."  '[voice 1]
[7D1A]       SAY "Commander !!! Commander !!! Another planetary coordinate ! You're doing a fine job , I gotta hand it to ya !"  '[skip 3]
[7D4C]       rec_0EB0 |= 0x2
[7D51]       secret = 0
[7D58]       trak21 = 1
    END
[7D5F]     SAY "Me like star dust on your shoulders , friend ... word_65535 bye_bye"  '[voice 5, skip 1]
[7D7F]     adieu = 1
[7D86]     IF-BLOCK (exit -> @7DE1)
[7D89]       GUARD adieu == 1
[7D90]       ENDIF
[7D91]       SAY "Before leave , you must go Observatory talk to my father Maxxon ..."  '[voice 5]
[7DB3]       SAY "Bye bye ... You come back soon ... Little Yoko wait you ..."  '[voice 3, skip 2]
[7DD5]       H1 = 1
[7DDC]       OP_C1 C1 4E 12 60 0B
    END
  END
[7DE1]   BLOCK (exit -> @7F76)
[7DE5]     AWAIT gameflag_252A
[7DE6]     GUARD rec_0F4E == 2858
[7DEB]     GUARD active_actor == Yoko.talk (related 40)
[7DF0]     GUARD H1 == 1
[7DF7]     ENDIF
[7DF8]     SAY "Hello , stranger . You back , Yoko happy ... Welcome to planet Rondo ..."  '[voice 1, skip 2]
[7E1E]     secret = 0
[7E25]     adieu = 0
[7E2C]     SAY "Yuk ! He's even uglier than he used to be , Commander . Is it a secret weapon ?"
[7E5A]     SAY "Me greet you , noble stranger ..."  '[voice 3, skip 1]
[7E70]     rec_0240 = 6
[7E75]     SAY "You want talk ?"  '[voice 4]
[7E85]     IF-BLOCK (exit -> @7F1F)
[7E88]       GUARD secret == 1
[7E8F]       ENDIF
[7E90]       SAY "Me know Slimers . Them live on nice planet Ekatomb, in jelly village ."  '[voice 6]
[7EB4]       SAY "Them be GLUXX family ... Them be very nice ..."  '[voice 9]
[7ED0]       SAY "Commander !!! Commander !!! Another planetary coordinate , you're my hero , Commander ..."
[7EF4]       SAY "Planet Ekatomb be in EKATOMBUS 435DRTX constellation ..."  '[voice 10, skip 3]
[7F0C]       rec_0EB0 |= 0x2
[7F11]       secret = 0
[7F18]       trak21 = 1
    END
[7F1F]     SAY "Me like star dust on your shoulders , friend ... word_65535 bye_bye"  '[voice 5, skip 1]
[7F3F]     adieu = 1
[7F46]     IF-BLOCK (exit -> @7F76)
[7F49]       GUARD adieu == 1
[7F50]       ENDIF
[7F51]       SAY "Bye bye ... You come back soon ... Little Yoko wait you ..."  '[voice 3, skip 1]
[7F73]       END PRESENTATION Yoko.talk
    END
  END
[7F76]   BLOCK (exit -> @807C)
[7F7A]     AWAIT gameflag_252A
[7F7B]     GUARD rec_0F4E == 2858
[7F80]     GUARD (rec_0EB0 & 0x2) != 0
[7F85]     GUARD active_actor == Yoko.talk (related 40)
[7F8A]     GUARD H1 == 1
[7F91]     ENDIF
[7F92]     SAY "You see his trunk , Commander ? Barf ! Ha! Ha! Ha! ..."
[7FB4]     SAY "You want talk ?"  '[voice 5]
[7FC4]     IF-BLOCK (exit -> @8028)
[7FC7]       GUARD mastok == 1
[7FCE]       GUARD rec_02A2 == 3932
[7FD3]       GUARD (rec_09E8 & 0x2) == 0
[7FD9]       ENDIF
[7FDA]       SAY "Look at the planet Mastachok, It be beat up..."  '[voice 2]
[7FF4]       SAY "Mastachok coordinates X453 Y321..."  '[voice 3, skip 1]
[8004]       rec_09E8 |= 0x2
[8009]       SAY "Commander ... he gave us another planet ...."  '[skip 1]
[8021]       mastok = 0
    END
[8028]     SAY "Me must go a few days , friend ... word_65535 bye_bye"  '[voice 5, skip 1]
[8046]     adieu = 1
[804D]     IF-BLOCK (exit -> @807C)
[8050]       GUARD adieu == 1
[8057]       ENDIF
[8058]       SAY "Bye bye ... See you in a few days ..."  '[voice 3, skip 2]
[8074]       rec_0212 = 3788
[8079]       END PRESENTATION Yoko.talk
    END
  END
[807C]   BLOCK (exit -> @8252)
[8080]     AWAIT gameflag_252A
[8081]     GUARD NOT rec_11B0 == 1298
[8087]     GUARD NOT rec_11B0 == 40
[808D]     GUARD active_actor == Maxxon.talk (related 40)
[8092]     GUARD I1 == 0
[8099]     ENDIF
[809A]     IF-BLOCK (exit -> @8179)
[809D]       GUARD rec_0548 == 1
[80A4]       ENDIF
[80A5]       SAY "Welcome stranger . Me not know you . You come from far ..."  '[voice 2]
[80C7]       SAY "Me Izwal Maxxon . Me be big astronomer . Me build big telescope for see stars ."  '[voice 3]
[80F1]       SAY "Me like study sky , stars , galaxies ... But me have big problem , friend ..."  '[voice 4]
[811B]       SAY "Telescope not have good lens . Me seek lens for telescope ..."  '[voice 5]
[813B]       SAY "With nice lens , me can see stars , galaxies , black holes ... Me big hobby , friend ..."  '[voice 6]
[816B]       SAY "You know ?"  '[voice 5]
    END
[8179]     IF-BLOCK (exit -> @819A)
[817C]       GUARD rec_0548 > 1
[8183]       ENDIF
[8184]       SAY "You're back ... Friend ... Welcome ..."  '[voice 3]
    END
[819A]     SAY "If you get lens , me do big favor for you ..."  '[voice 9]
[81BA]     SAY "If you hear of LENS , you tell me , Commander ..."  '[voice 8]
[81DA]     SAY "Commander , Commander , that telescope of his could help us find some black holes ... Smart thinking , huh ?"
[820C]     SAY "See you soon , friend ... You go now ... Me have much work ..."  '[voice 8, skip 1]
[8232]     rec_0B62 |= 0x2
[8237]     SAY "..."  '[skip 3]
[8241]     adieu = 0
[8248]     trak22 = 1
[824F]     END PRESENTATION Maxxon.talk
  END
[8252]   BLOCK (exit -> @82FA)
[8256]     AWAIT gameflag_252A
[8257]     GUARD active_actor == Maxxon.talk (related 40)
[825C]     GUARD rec_11B0 == 40
[8261]     ENDIF
[8262]     SAY "Welcome stranger ... You come back to planet Rondo ..."  '[voice 2]
[827E]     SAY "You have information on lens for telescope ?"  '[voice 6]
[8296]     SAY "I think he's gonna like this , Commander. Send him over his lens ..."
[82BA]     SAY "TELEPORT LENS TO OBSERVATORY : word_65535 teleport"
[82D2]     IF-BLOCK (exit -> @82FA)
[82D5]       GUARD concept == "teleport"
[82D8]       ENDIF
[82D9]       SAY "TELEPORTING LENS TO PLANET RONDO"  '[skip 3]
[82EB]       OP_CD CD 30 00 9C 11 12 05
[82F2]       lent = 1
[82F9]       CLEAR concept_alt
    END
  END
[82FA]   BLOCK (exit -> @842D)
[82FE]     AWAIT gameflag_252A
[82FF]     GUARD active_actor == Maxxon.talk (related 40)
[8304]     GUARD rec_11B0 == 1298
[8309]     GUARD lent == 1
[8310]     ENDIF
[8311]     SAY "THIS NOT RIGHT LENS , FRIEND !!!"  '[voice 5]
[8327]     SAY "THIS BE STRANGE JELLY LENS ..."
[833B]     SAY "No , no , no , Mister Maxxon ! Nobody uses glass lenses anymore !"
[8361]     SAY "Why don't you just try it ..."
[8377]     SAY "Holy snake oil , Commander . Don't they have any progress around here ? ... I mean, these Izwals are nice folks ,"
[83AD]     SAY "but don't count on them for inventing the wheel ..."
[83C9]     SAY "Me try lens . You come back later . Me thank you anyway for effort ..."  '[voice 8]
[83F1]     SAY "Bye bye , friend . See soon ..."  '[voice 9]
[8409]     PP1 = 1
[8410]     SAY "Bye bye"  '[skip 3]
[841C]     trak23 = 1
[8423]     lent = 2
[842A]     END PRESENTATION Maxxon.talk
  END
[842D]   BLOCK (exit -> @85D6)
[8431]     AWAIT gameflag_252A
[8432]     GUARD active_actor == Maxxon.talk (related 40)
[8437]     GUARD rec_11B0 == 1298
[843C]     GUARD lent == 2
[8443]     ENDIF
[8444]     SAY "LENS BE RIGHT LENS , FRIEND !!!"  '[voice 5]
[845A]     SAY "Very nice technology ..."
[846A]     SAY "You see , Mister Maxxon ! Glass lenses went out with gunpowder and real food ..."
[8492]     SAY "You have to try things out before you start moaning ..."
[84B0]     IF-BLOCK (exit -> @84C1)
[84B3]       GUARD NOT rec_1018 == 40
[84B9]       ENDIF
[84BA]       pi = 1
    END
[84C1]     IF-BLOCK (exit -> @84D1)
[84C4]       GUARD rec_1018 == 40
[84C9]       ENDIF
[84CA]       pi = 0
    END
[84D1]     IF-BLOCK (exit -> @8559)
[84D4]       GUARD pi == 1
[84DB]       ENDIF
[84DC]       SAY "To reward you , me give you a CRED, friend..."  '[voice 2]
[84F8]       SAY "Wow ! He's giving us a whole CRED ... I knew it, Commander. They're richer than Cap'n Bob..."
[8524]       SAY "TELEPORT CRED IN CRYOBOX word_65535 teleport"
[853A]       IF-BLOCK (exit -> @8559)
[853D]         GUARD concept == "teleport"
[8540]         ENDIF
[8541]         SAY "TELEPORT CRED IN CRYOBOX"  '[skip 2]
[8551]         OP_CD CD CC 00 04 10 28 00
[8558]         CLEAR concept_alt
      END
    END
[8559]     SAY "Me set up lens , me not have time idle talk . You come back in moment ..."  '[voice 8]
[8585]     SAY "Okay . We'll come back when you've got it all set up ..."
[85A7]     SAY "Me much work . Bye bye , friend . See soon ..."  '[voice 9]
[85C7]     SAY "Bye bye..."  '[voice 2, skip 1]
[85D3]     END PRESENTATION Maxxon.talk
  END
[85D6]   BLOCK (exit -> @85EF)
[85DA]     GUARD rec_1018 == 40
[85DF]     GUARD rec_11B0 == 1298
[85E4]     ENDIF
[85E5]     rec_052A = 3788
[85EA]     rec_0212 = 3788
  END
[85EF]   BLOCK (exit -> @8852)
[85F3]     AWAIT gameflag_252A
[85F4]     GUARD J1 == 0
[85FB]     GUARD rec_0F4E == 3758
[8600]     GUARD active_actor == Daddy_Gluxx.talk (related 40)
[8605]     ENDIF
[8606]     SAY "Ho! A visitor from far away in the space-time continuum ..."  '[voice 1, skip 1]
[8624]     adieu = 0
[862B]     SAY "See , my children , how fascinating he is of aspect ..."  '[voice 2]
[864B]     SAY "Ooohh , such extraordinary beauty ..."  '[voice 3]
[865F]     SAY "Magnificence of Nature..."  '[voice 4]
[866D]     SAY "Ha! Ha! I'm Daddy GLUXX this is my family , noble stranger ..."  '[voice 2]
[868F]     SAY "We're SLIMERS. This jelly village is where we live ..."  '[voice 1]
[86AB]     SAY "We have some cousins ... old SLIM GELATI and his wife, they're trying to recapture their youth ..."  '[voice 0]
[86D7]     SAY "Slim Gelati and his wife . They're retired now . They long for eternal youth !"  '[voice 4]
[86FF]     SAY "Ha! Ha! The old always dream of being young. Ha! Ha! Ha!"  '[voice 2]
[871F]     SAY "But youth costs a fortune , you know ... Poor old Slim... He sold everything he owned ..."  '[voice 2]
[874B]     SAY "What do you seek , stranger ?"  '[voice 4, skip 1]
[8761]     rec_0480 = 1
[8766]     SAY "Bye bye , stranger . Come back again ... word_65535 bye_bye"  '[voice 5, skip 1]
[8784]     adieu = 1
[878B]     IF-BLOCK (exit -> @882A)
[878E]       GUARD secret == 1
[8795]       ENDIF
[8796]       SAY "I know of a terrible place ... Nobody speaks of it ..."  '[voice 2]
[87B6]       SAY "It's the clinic on the planet Erazor . Mysterious things happen there , you know ..."  '[voice 2]
[87DE]       SAY "Do not venture near that place ... ERAZORUS AS666 constellation...."  '[voice 2]
[87FA]       SAY "Commander ... Commander ... Unless I'm mistaken , that sounded like planetary coordinates !"  '[skip 2]
[881E]       rec_097C |= 0x2
[8823]       trak25 = 1
    END
[882A]     IF-BLOCK (exit -> @8852)
[882D]       GUARD adieu == 1
[8834]       ENDIF
[8835]       SAY "Bye bye"  '[skip 3]
[8841]       trak24 = 1
[8848]       J1 = 1
[884F]       END PRESENTATION Daddy_Gluxx.talk
    END
  END
[8852]   BLOCK (exit -> @8997)
[8856]     AWAIT gameflag_252A
[8857]     GUARD J1 == 1
[885E]     GUARD rec_0F4E == 3758
[8863]     GUARD active_actor == Daddy_Gluxx.talk (related 40)
[8868]     ENDIF
[8869]     SAY "Ho! A visitor from far away in the space-time continuum ..."  '[voice 1, skip 2]
[8887]     adieu = 0
[888E]     secret = 0
[8895]     SAY "You have come back , stranger . You are welcome here ..."  '[voice 0]
[88B5]     SAY "What knowledge do you seek ?"  '[voice 3, skip 1]
[88C9]     rec_0480 = 1
[88CE]     IF-BLOCK (exit -> @897A)
[88D1]       GUARD secret == 1
[88D8]       GUARD (rec_097C & 0x2) == 0
[88DE]       GUARD trak25 == 0
[88E5]       ENDIF
[88E6]       SAY "I know of a terrible place ... Nobody speaks of it ..."  '[voice 2]
[8906]       SAY "It's the clinic on the planet Erazor . Mysterious things happen there , you know ..."  '[voice 2]
[892E]       SAY "Do not venture near that place ... ERAZORUS AS666 constellation...."  '[voice 2]
[894A]       SAY "Commander ... Commander ... Unless I'm mistaken , that sounded like planetary coordinates !"  '[skip 2]
[896E]       rec_097C |= 0x2
[8973]       trak25 = 1
    END
[897A]     SAY "Bye bye , sir ... word_65535 bye_bye"  '[voice 3, skip 2]
[8990]     POKE [0x8998] = 1
[8994]     END PRESENTATION Daddy_Gluxx.talk
  END
[8997]   GOTO @89AD
[899B]   IF-BLOCK (exit -> @89AD)
[899E]     GUARD (rec_097C & 0x2) != 0
[89A3]     ENDIF
[89A4]     rec_0452 = 3788
[89A9]     POKE [0x8998] = 0
  END
[89AD]   BLOCK (exit -> @8D67)
[89B1]     AWAIT gameflag_252A
[89B2]     GUARD rec_0F4E == 2426
[89B7]     GUARD active_actor == Otto_Von_Smile.talk (related 40)
[89BC]     GUARD P1 == 0
[89C3]     ENDIF
[89C4]     SAY "Welcome , dear dear client . Take a seat . Our 3 star clinic is happy to have you ..."  '[voice 3, skip 1]
[89F4]     trak26 = 1
[89FB]     SAY "I am the well-known Doctor OTTO VON SMILE . Why not be happy , since I can make life last for ever ... Ha! Ha! Ha!"  '[voice 2]
[8A37]     SAY "Commander, this guy doesn't feel right to me ... Stay alert ..."
[8A57]     SAY "You seek youth through the ageless power of plants ? A new and vibrant body organ perhaps ? A little plastic surgery ? Here , all is possible !"  '[voice 4]
[8A99]     SAY "Yuk !!! Let's get out of here , Commander..."
[8AB3]     SAY "What miracles may I perform for you , dear dear client ? word_65535 grow_young transplant surgery"  '[voice 5]
[8ADD]     IF-BLOCK (exit -> @8BA6)
[8AE0]       GUARD concept == "grow_young"
[8AE3]       ENDIF
[8AE4]       SAY "Ha! Ha! Youth ! How exquisitely unoriginal ... And so wildly expensive ! Tell me , do you have 200,000 creds ?"  '[voice 6]
[8B18]       SAY "No ? You are POOR ? How ghastly for you . ONLY THE RICH MAY TASTE ETERNITY !!"  '[voice 7]
[8B44]       SAY "What a geek , Commander... Maybe we should wake Cap'n Bob and tell him about eternity ..."
[8B6E]       SAY "Another time perhaps , dear client and , dare I hope , friend ! Ha! Ha! Ha!..."  '[voice 2]
[8B98]       SAY "stop"  '[skip 2]
[8BA2]       CLEAR concept_alt
[8BA3]       END PRESENTATION Otto_Von_Smile.talk
    END
[8BA6]     IF-BLOCK (exit -> @8CB6)
[8BA9]       GUARD concept == "transplant"
[8BAC]       ENDIF
[8BAD]       SAY "Ha! Ha! A new and vibrant retina ? Well , I have a simply gigantic selection to choose from ..."  '[voice 5]
[8BDD]       SAY "For those who desire to see far or near or very near indeed ! Ha! Ha! These are lenses of the very highest quality ..."  '[voice 6]
[8C17]       SAY "Each sells for the paltry sum of 2000 creds ... A snip at the price ..."  '[voice 7]
[8C3F]       SAY "Commander , that's exactly what Mister MAXXON needs for his telescope ..."
[8C5F]       SAY "But you do not have 2000 creds , do you ? I find the poor so tasteless !"  '[voice 3]
[8C8B]       SAY "Another time perhaps , dear client ..."  '[voice 2]
[8CA1]       SAY "stop"  '[skip 3]
[8CAB]       CLEAR concept_alt
[8CAC]       P1 = 1
[8CB3]       END PRESENTATION Otto_Von_Smile.talk
    END
[8CB6]     IF-BLOCK (exit -> @8D67)
[8CB9]       GUARD concept == "surgery"
[8CBC]       ENDIF
[8CBD]       SAY "Ha! Ha! A little operation ! Just like everybody else ... It's divinely expensive , naturally ... Do you have 100,000 creds ?"  '[voice 2]
[8CF3]       SAY "No , you don't have such a sum ... Well run along and get it ..."  '[voice 5]
[8D1B]       SAY "What a slimeball , Commander..."
[8D2D]       SAY "Another time perhaps , dear client and , dare I hope , friend ! Ha! Ha! Ha! ..."  '[voice 6]
[8D59]       SAY "stop"  '[skip 7]
[8D63]       CLEAR concept_alt
[8D64]       END PRESENTATION Otto_Von_Smile.talk
    END
  END
[8D67]   BLOCK (exit -> @8F3C)
[8D6B]     AWAIT gameflag_252A
[8D6C]     GUARD rec_0F4E == 2426
[8D71]     GUARD active_actor == Otto_Von_Smile.talk (related 40)
[8D76]     GUARD P1 == 1
[8D7D]     ENDIF
[8D7E]     SAY "Well hello , dear client . Do take a seat and admire our 3 star clinic !"  '[voice 2]
[8DA8]     SAY "I am as you probably know the admired and respected Doctor OTTO VON SMILE... Ha! Ha! Ha!"  '[voice 4]
[8DD2]     SAY "So you are interested in optical lenses ... I see ! So to speak ..."  '[voice 5]
[8DF8]     SAY "Allow me to propose a deal ... I would like to count SLIMERS among my dear clientele . You wouldn't happen to know of any Slimer planets ? ..."  '[voice 6]
[8E3A]     SAY "Why don't you tell me the name of a Slimer planet and I'll give you a lovely little lens . Just one lens ... You'll have to pay for the other ... Ha! Ha! ha!"  '[voice 3]
[8E88]     SAY "I'm waiting ... word_65535 Corpo Ekatomb Kult Troma Cyberock Moskito Tumul"  '[voice 4]
[8EA8]     IF-BLOCK (exit -> @8EB7)
[8EAB]       GUARD concept == "Ekatomb"
[8EAE]       ENDIF
[8EAF]       eka = 1
[8EB6]       CLEAR concept_alt
    END
[8EB7]     IF-BLOCK (exit -> @8EC0)
[8EBA]       GUARD NOT concept == "Ekatomb"
[8EBE]       ENDIF
[8EBF]       CLEAR concept_alt
    END
[8EC0]     SAY "I'll just send some of my drones to verify what you claim ..."  '[voice 2]
[8EE2]     SAY "If you've told me the truth , why , then you shall have your little lens ..."  '[voice 7]
[8F0C]     SAY "Bye bye for the present , dear dear client ..."  '[voice 5]
[8F28]     SAY "stop"  '[skip 2]
[8F32]     P1 = 2
[8F39]     END PRESENTATION Otto_Von_Smile.talk
  END
[8F3C]   BLOCK (exit -> @90A6)
[8F40]     AWAIT gameflag_252A
[8F41]     GUARD rec_0F4E == 2426
[8F46]     GUARD active_actor == Otto_Von_Smile.talk (related 40)
[8F4B]     GUARD P1 == 2
[8F52]     GUARD eka == 1
[8F59]     ENDIF
[8F5A]     SAY "Well hello , dear client . Do take a seat and admire our 3 star clinic !"  '[voice 2]
[8F84]     SAY "I am the immensely gifted Doctor OTTO VON SMILE... Ha! Ha! Ha!"  '[voice 5]
[8FA4]     SAY "You want optical lens ... I see , so to speak !"  '[voice 6]
[8FC4]     SAY "The little expedition is winging its way back even now ..."  '[voice 3]
[8FE2]     SAY "Your information proved quite accurate . Well done ! Thanks to you , our Slimer friends shall soon be able to avail of my services !"  '[voice 0]
[901E]     SAY "And now , as promised , one lens !"  '[voice 2]
[9038]     SAY "TELEPORT LENS TO ARK : word_65535 teleport"
[9050]     IF-BLOCK (exit -> @9071)
[9053]       GUARD concept == "teleport"
[9056]       ENDIF
[9057]       SAY "LENS TELEPORTED TO CRYOBOX ."  '[skip 2]
[9069]       OP_CD CD 24 06 9C 11 28 00
[9070]       CLEAR concept_alt
    END
[9071]     SAY "thank you and goodbye , dear DEAR client ..."  '[voice 1]
[908B]     SAY "stop"  '[skip 3]
[9095]     P1 = 4
[909C]     trak27 = 1
[90A3]     END PRESENTATION Otto_Von_Smile.talk
  END
[90A6]   BLOCK (exit -> @9247)
[90AA]     AWAIT gameflag_252A
[90AB]     GUARD rec_0F4E == 2426
[90B0]     GUARD active_actor == Otto_Von_Smile.talk (related 40)
[90B5]     GUARD P1 == 2
[90BC]     GUARD eka == 0
[90C3]     ENDIF
[90C4]     SAY "Well hello , dear client . Do take a seat and admire our 3 star clinic !"  '[voice 0]
[90EE]     SAY "As you no doubt know , I am none other than Doctor OTTO VON SMILE... Ha! Ha! Ha!"  '[voice 1]
[911A]     SAY "And you are interested in optical lenses... Yes , I can see that quite clearly !"  '[voice 2]
[9142]     SAY "The little expedition is winging its way back even now ..."  '[voice 3]
[9160]     SAY "You were quite mistaken , dear friend . The planet you mentioned is Slimerless !"  '[voice 4]
[9186]     SAY "So , am I to have the name of the planet ? word_65535 Corpo Ekatomb Kult Troma Cyberock Moskito Tumul"  '[voice 5]
[91B8]     IF-BLOCK (exit -> @91C7)
[91BB]       GUARD concept == "Ekatomb"
[91BE]       ENDIF
[91BF]       eka1 = 1
[91C6]       CLEAR concept_alt
    END
[91C7]     IF-BLOCK (exit -> @91D0)
[91CA]       GUARD NOT concept == "Ekatomb"
[91CE]       ENDIF
[91CF]       CLEAR concept_alt
    END
[91D0]     SAY "I'll just send some more of my drones to verify what you claim ..."  '[voice 2]
[91F4]     SAY "If you've told me the truth , then you shall have your little lens ..."  '[voice 0]
[921A]     SAY "Bye bye for the present , dear dear client ..."  '[voice 1]
[9236]     SAY "stop"  '[skip 2]
[9240]     POKE [0x9248] = 1
[9244]     END PRESENTATION Otto_Von_Smile.talk
  END
[9247]   GOTO @925A
[924B]   GUARD eka1 == 1
[9252]   ENDIF
[9253]   eka = 1
[925A]   BLOCK (exit -> @930B)
[925E]     AWAIT gameflag_252A
[925F]     GUARD rec_0F4E == 2426
[9264]     GUARD active_actor == Otto_Von_Smile.talk (related 40)
[9269]     GUARD NOT rec_11B0 == 1514
[926F]     ENDIF
[9270]     SAY "Hello , dear client . Please don't sit ..."  '[voice 2]
[928A]     SAY "The astonishing Doctor OTTO VON SMILE is very busy indeed ... Ha! Ha! Ha!"  '[voice 5]
[92AE]     SAY "I simply cannot help you for the moment ..."  '[voice 6]
[92C8]     SAY "Do come back another time ..."  '[voice 3]
[92DC]     SAY "Thank you once more for your delightful visit , dear DEAR client ..."  '[voice 1]
[92FE]     SAY "stop"  '[skip 1]
[9308]     END PRESENTATION Otto_Von_Smile.talk
  END
[930B]   BLOCK (exit -> @9323)
[930F]     GUARD rec_02C0 == 1
[9316]     ENDIF
[9317]     state[7] = 20
[931B]     OP_B7 B7 B0 00 02
[931F]     POKE [0x930C] = 0
  END
[9323]   BLOCK (exit -> @9340)
[9327]     GUARD state[7] == 0
[9329]     ENDIF
[932A]     rec_0094 |= 0x2
[932F]     OP_C3 C3 CC 00 28 00
[9334]     state[7] = 65535
[9338]     POKE [0x9341] = 1
[933C]     POKE [0x9324] = 0
  END
[9340]   GOTO @93E8
[9344]   AWAIT presentation
[9345]   GUARD active_actor == Bug_Deluxe.talk (related 40)
[934A]   ENDIF
[934B]   SAY "COMMERCIAL ... COMMERCIAL ... COMMERCIAL ..."  '[skip 1]
[935F]   SETCHAR slot 3 = "venus"
[9368]   SAY "FOR ALL SPENDING MOMENTS , COME TO VENUSIA"
[9380]   SAY "FABULOUS BARGAINS ... BRING PLENTY OF CREDS..."
[9396]   SAY "MORE FOR YOUR CREDS AT VENU-USIA-AA ... Watch tv channel three ..."
[93B6]   SAY "VENUSIA 325467 DEGREES, 23 GALAXY B BABY1..."
[93CC]   SAY "CRUIK..........."
[93D6]   SAY "stop"  '[skip 2]
[93E0]   rec_0922 |= 0x2
[93E5]   END PRESENTATION Bug_Deluxe.talk
[93E8]   BLOCK (exit -> @9486)
[93EC]     GUARD rec_0F4E == 2336
[93F1]     GUARD active_actor == Bug_Deluxe.talk (related 40)
[93F6]     ENDIF
[93F7]     SAY "Hi there , Comsumer Comrade . Welcome to VENUSIA SUPRAMARKET"  '[voice 1]
[9413]     SAY "Wanna buy a SUPRA product at VENUSIA? word_65535 yes no"  '[voice 4]
[9431]     IF-BLOCK (exit -> @943D)
[9434]       GUARD concept == "yes"
[9437]       ENDIF
[9438]       POKE [0x9487] = 1
[943C]       CLEAR concept_alt
    END
[943D]     IF-BLOCK (exit -> @9486)
[9440]       GUARD concept == "no"
[9443]       ENDIF
[9444]       SAY "NO ! HA HA!! Forgot our CREDs , did we ?"  '[voice 6]
[9462]       SAY "Have a happy day , comrade ."  '[voice 2]
[9478]       SAY "stop"  '[skip 2]
[9482]       CLEAR concept_alt
[9483]       END PRESENTATION Bug_Deluxe.talk
    END
  END
[9486]   GOTO @967C
[948A]   GUARD rec_0F4E == 2336
[948F]   AWAIT gameflag_252A
[9490]   GUARD active_actor == Bug_Deluxe.talk (related 40)
[9495]   ENDIF
[9496]   SAY "You got the CREDs? At VENUSIA , it's CREDs CREDs CREDs , Consumer Comrade ..."  '[voice 5]
[94BC]   IF-BLOCK (exit -> @950B)
[94BF]     GUARD PP1 == -1.value
[94C6]     GUARD ach == -1.value
[94CD]     GUARD NOT rec_1018 == 40
[94D3]     ENDIF
[94D4]     SAY "No CREDs , huh ?... That's not an attitude we care for at VENUSIA !"
[94FA]     SAY "stop"  '[skip 2]
[9504]     POKE [0x9487] = 0
[9508]     END PRESENTATION Bug_Deluxe.talk
  END
[950B]   IF-BLOCK (exit -> @95DE)
[950E]     GUARD PP1 == 1
[9515]     GUARD NOT rec_1018 == 40
[951B]     ENDIF
[951C]     SAY "It's your luck-lucky day , consumer comrade . You just won a free CRED !!!"  '[voice 5, skip 1]
[9542]     LOADSTR "pubven1.hnm"
[9550]     SAY "TELEPORT CRED TO ARK word_65535 teleport"
[9566]     IF-BLOCK (exit -> @95DE)
[9569]       GUARD NOT concept == "teleport"
[956D]       ENDIF
[956E]       SAY "CRED TELEPORTED TO ARK"  '[skip 1]
[957E]       OP_CD CD CC 00 04 10 28 00
[9585]       SAY "Ahhh ! Lady luck loves a consumer , comrade ... Happy spending !"  '[voice 5]
[95A7]       SAY "And remember ... "MORE FOR YOUR CREDS AT VENU-USIA-AAA !""  '[voice 4]
[95C3]       SAY "Bye bye comrade consumer..."  '[voice 5, skip 3]
[95D3]       PP1 = -1.value
[95DA]       CLEAR concept_alt
[95DB]       END PRESENTATION Bug_Deluxe.talk
    END
  END
[95DE]   IF-BLOCK (exit -> @9610)
[95E1]     GUARD rec_1018 == 40
[95E6]     ENDIF
[95E7]     SAY "What would you like to splurge your creds on , Supra Comrade?"  '[voice 2, skip 2]
[9607]     state[7] = 500
[960B]     rec_00D8 = 15302
  END
[9610]   SAY "TELEPORT CRED ON VENUSIA ... word_65535 buy"  '[skip 1]
[9626]   LOADSTR "pion.hnm"
[9631]   IF-BLOCK (exit -> @967C)
[9634]     GUARD ach == 1
[963B]     ENDIF
[963C]     SAY "Thank you for enjoying VENUSIA the SUPRAMARKET . VENUSIA hopes to see you again soon ..."  '[voice 4]
[9664]     SAY "stop"  '[skip 3]
[966E]     ach = -1.value
[9675]     POKE [0x9487] = 0
[9679]     END PRESENTATION Bug_Deluxe.talk
  END
[967C]   BLOCK (exit -> @96AA)
[9680]     GUARD (rec_0AF0 & 0x2) != 0
[9685]     GUARD C1 == 6
[968C]     GUARD rec_11B0 == 1298
[9691]     GUARD rec_0722 == 65535
[9696]     GUARD rec_0332 == 65535
[969B]     ENDIF
[969C]     rec_06C4 |= 0x2
[96A1]     OP_C3 C3 FC 06 28 00
[96A6]     POKE [0x96AB] = 1
  END
[96AA]   GOTO @9881
[96AE]   AWAIT presentation
[96AF]   GUARD active_actor == Scruter_K.talk (related 40)
[96B4]   ENDIF
[96B5]   SAY "This is SCRUT customs and excise ship ... Calling unknown vessel ..."
[96D5]   SAY "Pull over for a routine check ... We have orders to routinely check your ship specifically ..."
[96FF]   SAY "Scanning X-ray Charles ... BZZZZZZZZZZZZZZZZZZZZZZ ..."
[9713]   SAY "We are currently scanning your vessel ..."
[9729]   SAY "Bzzzzzzzzzzzzzzzzzz"
[9733]   IF-BLOCK (exit -> @975F)
[9736]     GUARD rec_1018 == 65535
[973B]     ENDIF
[973C]     SAY "ILLEGAL MONEY DETECTED ..."  '[skip 1]
[974C]     OP_CD CD 30 00 04 10 C2 06
[9753]     SAY "Confiscated ..."
  END
[975F]   IF-BLOCK (exit -> @9791)
[9762]     GUARD rec_11C8 == 65535
[9767]     ENDIF
[9768]     SAY "BIONIUM container detected ... Ultra-illegal item ..."  '[skip 1]
[977E]     OP_CD CD 30 00 B4 11 C2 06
[9785]     SAY "Confiscated ..."
  END
[9791]   IF-BLOCK (exit -> @97CE)
[9794]     GUARD rec_1048 == 65535
[9799]     ENDIF
[979A]     SAY "Pirate decoder ... You're a cheater ..."  '[skip 1]
[97B0]     SAY "Confiscated ..."
[97BC]     SAY "It does'nt work... No confiscated..."
  END
[97CE]   SAY "Your documentation isn't in order ... Everything is confiscated ..."
[97EA]   SAY "Commander, Commander , this could be bad in survival terms ... What's Cap'n Bob gonna say ?"
[9814]   SAY "Be happy ... You're alive ... Ha! Ha! Ha! ..."
[9830]   SAY "Kruiiik !!!"
[983C]   SAY "Holy snake-oil ..."
[984A]   SAY "They left us right in the middle of this star soup ... How about that ..."
[9872]   SAY "..."  '[skip 2]
[987C]   RUN PROFILE 2
[987E]   END PRESENTATION Scruter_K.talk
[9881] END OF SCRIPT

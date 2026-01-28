
User-reported issue:
    - currently the audio playback is good when the video is opened in the editor. 
    - The speed is good. No issue there.
    - This has to be maintined. 
    - However, the issue is that when I delete a segment the audio around that point never re-loads.
    - It's been whack-a-mole: audio loads at the start but then deleting segments caused sync issues. then we fixed the sync issues but then it took ~7 seconds / 10 minutes of footage for the audio to load. then on deletion it also too take long as it was re-loading the whole audio
    - then we fixed the reloading/loading issue - but now (even though logically it might be insync) the buffer is gone - so the audio is gone once I delete a segement.
    - We need all: i) fast preloading on opening the file ii) keeping segments sycned on deletion iii) fast re-loading (eg. buffer the next and prevoius 20 seconds -> 40 seconds -> 80 seonds etc.) until it's all reloaded.
    - strict user flow: fast loading on opening, near-instant loading whilst synced on segment deletion (not segment might be self-contained reocridng segment OR i o marked segments marked)

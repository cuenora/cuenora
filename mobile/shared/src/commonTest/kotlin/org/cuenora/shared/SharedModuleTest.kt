package org.cuenora.shared

import kotlin.test.Test
import kotlin.test.assertEquals

class SharedModuleTest {
    @Test
    fun exposesSharedLibraryName() {
        assertEquals("CuenoraShared", SharedModule.name)
    }
}
